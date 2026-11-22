# Implementation Plan: Target Search Finds Names of Several Words

**Branch**: `049-search-finds-whole-names` | **Date**: 2026-11-22 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

`Catalog::search` stops comparing the whole query with each word of a common name. A
private `words` makes the words of the query and of each name alike: the pieces between
spaces, lowercased, each kept to its letters and digits. A private `name_matches` says a
name matches when each query word begins one of the name's words, in any order, or when
the query's words run together read along the name from the start of one of its words
(spec owner decisions 1 and 2). A query with no word finds nothing. So no mark and no
space has to be typed as the name has it, and no mark is named in the code. An id is
looked up as before, marks and all. A query of one word that holds a letter or a digit
matches at least the objects it did; five of today's 951 one-word prefixes match more,
and `&` finds nothing. The exact id, the order by magnitude and the limit are as they
are. `targets::search`'s doc comment says the rule. `GET /api/targets` and the MCP tool
`search_targets` answer through the same function, so both find "Orion Nebula" with no
change of their own.

One story: the rule, `words`, `name_matches`, the doc comment, nine unit tests after the
last test of the module that holds 012's search tests, and one test module at the MCP
door that asks the route and the tool alike.

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

**Scale/Scope**: one story; one changed function and doc comment, two private
functions, one new test module, one `mod` line.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, star detection or `crates/math-core`. The expectations are read from the pinned OpenNGC extract, the catalog's own source, and from 012's rule as its FR-007 words it. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. The new unit tests are new functions after the module's last; 012's search tests and 016's search-tool test pass unchanged (§ Shapes that existing tests pin). |
| III. Tests first, independent expectations | Every expected list is written in the test from the extract's rows, or computed by the test from the extract's text (US1-S5 to US1-S7), US1-S6's lower bound by 012's documented rule and its gains written as literals, never by calling `search` or its private functions; `targets::find` and `targets::all`, unchanged by FR-003, name the object a row or id stands for. The tool is held to the route's own answer. Eight of the ten scenarios fail on `main`'s code; the two that pass there pin what must not change (§ What was run and what was read). |
| IV. Critical paths | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, no `build.rs`. |
| V. Proven open source first, honest licensing | Considered by name and not tried: a full-text index (`tantivy`), fuzzy matchers (`nucleo`, `fuzzy-matcher`), edit distance (`strsim`) and Unicode folding (`unicode-normalization`, `deunicode`). Each would be a new direct dependency of `finder` (`strsim` is in `Cargo.lock` already, as another crate's) for a rule of a few lines over 182 names, all ASCII, and fuzzy matching would change what a word of one finds, against spec owner decision 1 (e). No reused code, no new license. |

## Project Structure

```text
crates/finder/
├── src/targets.rs                           # US1: the rule in Catalog::search; words; name_matches; search's doc comment; nine unit tests
└── tests/mcp/
    ├── main.rs                              # US1: mod target_words;
    └── target_words.rs                      # US1, new
```

## Interface

### `crates/finder/src/targets.rs` (US1)

In `Catalog::search` (`crates/finder/src/targets.rs:164`), the check for an empty query
(`crates/finder/src/targets.rs:166`) becomes:

```rust
        // Names are matched by letters and digits only, and every id holds one, so a
        // query without any finds nothing.
        let terms = words(&query);
        if terms.is_empty() || limit == 0 {
            return Vec::new();
        }
```

and the filter (`crates/finder/src/targets.rs:173` to `:184`) becomes:

```rust
        let mut matches: Vec<_> = self
            .objects
            .iter()
            .filter(|object| {
                exact.is_some_and(|exact| std::ptr::eq(*object, exact))
                    || object
                        .common_names
                        .iter()
                        .any(|name| name_matches(name, &terms))
            })
            .collect();
```

Before `fn catalog()` (`crates/finder/src/targets.rs:201`):

```rust
/// The words of `text`, lowercased, each reduced to its letters and digits.
fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|word| word.chars().filter(|c| c.is_alphanumeric()).collect())
        .filter(|word: &String| !word.is_empty())
        .collect()
}

/// Whether each of `terms` begins a word of `name`, in any order, or the terms run
/// together read straight along the name from the start of one of its words.
fn name_matches(name: &str, terms: &[String]) -> bool {
    let words = words(name);
    let typed = terms.concat();
    let each_begins_a_word = terms
        .iter()
        .all(|term| words.iter().any(|word| word.starts_with(term.as_str())));
    each_begins_a_word || (0..words.len()).any(|from| words[from..].concat().starts_with(&typed))
}
```

The trim, the lowercase, the zero-limit answer, the exact id (looked up from the
trimmed, lowercased query), the sort and the truncation are unchanged. The doc comment
of `pub fn search` (`crates/finder/src/targets.rs:222` to `:225`) becomes:

```rust
/// Exact ids first, then common names in brightness order.
///
/// In a common name only letters and digits are compared, whatever their case. A name
/// matches when each word of the query begins one of its words, in any order, or when
/// the query reads straight along the name from the start of one of its words. Objects
/// without a magnitude come last; canonical id breaks equal-magnitude ties. At most
/// `limit` objects are returned, with no duplicates.
```

The unit tests follow the module's last test,
`every_source_object_is_reachable_by_its_raw_and_canonical_id`
(`crates/finder/src/targets.rs:454`), with one helper of their own that maps a search's
objects to their ids, `fn ids(matches: Vec<&Target>) -> Vec<&str>`. The tests' names
are the builder's to choose. US1-S6's queries are built from the extract's `common_names`
column split on commas and spaces, as 012's rule reads words, each word lowercased and
cut after each of its characters; the test keeps those holding a letter or a digit by
its own check (`char::is_alphanumeric` on the query's characters, not `words`); its
oracle is 012's rule on the extract's rows, as a lower bound, and the gains spec.md
lists, written as literals in a `match` on the query.

### `crates/finder/tests/mcp/target_words.rs` (US1)

A module comment naming its oracles (the extract's common names, with the ids written in
the test, and the route's JSON), then one `#[tokio::test]` that builds the console router
with `support::mcp::app(support::mcp::state(), "127.0.0.1")` and, for each query of
US1-S8, sends `GET /api/targets?q=` with the query encoded as spec.md gives it (each
space as `+`) through `support::mcp::send`, reads it with `support::mcp::body`, and
sends `support::mcp::call("search_targets", json!({"q": query}))`. The route's ids are
the `id` strings of the elements of its JSON array. The tool's answer is not an error
when the JSON-RPC reply has no `error` member and its `result.isError` is not `true`,
as `crates/finder/tests/mcp/session_tools.rs:36` and
`crates/finder/tests/mcp/session_tools.rs:41` read them. It uses only
`crate::support::mcp`, `axum`, and `serde_json`.

### `crates/finder/tests/mcp/main.rs` (US1)

`mod target_words;` after `mod steering;` (`crates/finder/tests/mcp/main.rs:21`).

Nothing else changes.

## What was run and what was read

**Run on 2026-11-22**, on the branch `probe/049-letters`, rebased on `main` at 28f436df,
three times each with the same outcome unless said. The probe's tip, a9211056, holds the
ten scenarios' tests as spec.md gives them and the trial fix of § Interface. Its last
commit added the two cases probe round 1 asked for; the runs on `main`'s search code
and of the first sixteen wrong variants below were made one commit earlier, at
1580e503, and the first was repeated once at the tip with the same outcome. Of the two
checks § Interface gives for "not an error", the tip's test of US1-S8 makes the
`isError` one only; probe round 1's makes both, and passes.

- the tip's tests with `main`'s search code in place of the fix's (everything of
  `targets.rs` above its tests as on `main`): US1-S1, US1-S3, US1-S4, US1-S5, US1-S6,
  US1-S8, US1-S9 and US1-S10 fail; US1-S2 and US1-S7 pass, as they must, since `main`
  finds no name for any query of several words or for `tie`, and finds an id with a
  space alone; US1-S10 fails there by `&`, which finds C 14; 012's ten unit tests of
  the module pass;
- the tip's tests with the fix: all ten pass, and the module's ten other unit tests
  pass unchanged;
- with the fix, both gates' Rust commands, once at a9211056: `lint`'s four and `test`'s
  two, every test of the workspace passing (1,453 under nextest), and the doctests;
- eighteen wrong variants of the rule, each once, on the module's unit tests and
  US1-S8. Each fails at least one scenario:

  | Variant | Fails |
  |---|---|
  | No reading along | US1-S4, US1-S6, US1-S8, US1-S9 |
  | Reading along alone (spec owner decision 2 (c)) | US1-S1, US1-S3, US1-S9 |
  | A hyphen kept as if a letter | US1-S9 |
  | `'` kept | US1-S6, US1-S8, US1-S9 |
  | `’` kept | US1-S8, US1-S9 |
  | ASCII letters and digits only | US1-S2 |
  | A name's and a query's words also split at hyphens | US1-S2, US1-S6 |
  | The old check, an empty query only | US1-S10 |
  | Reading along from the name's first word only | US1-S4, US1-S9 |
  | Reading along over at most two of the name's words, the first of them included | US1-S9 |
  | The joined query found anywhere in the joined name, not only from the start of a word | US1-S2, US1-S6, and 012's `search_uses_name_word_prefixes_and_orders_by_magnitude_with_unknowns_last` |
  | Any query word rather than each | US1-S1 to US1-S4, US1-S7 to US1-S9 |
  | In "each word begins a word" only, a query word equal to a name word, not beginning it | US1-S1, US1-S3, US1-S9 |
  | The words in the name's order (decision 1 (b)) | US1-S1, US1-S3 |
  | Each query word a different name word (decision 1 (f)) | US1-S3 |
  | All of an object's names read as one (decision 1 (d)) | US1-S2, US1-S3 |
  | Words split at the space character only, not at any whitespace | US1-S1 |
  | A repeated query word dropped | US1-S2 |

**Run in probe round 1, on 2026-11-22**, on the branch `probe/049-r1` from be52f8a0
(`main` at 28f436df and this trio as it then stood), by a builder that read only the
trio: the story built as written, its files exactly the Refinement Record's `writes`.
§ Interface's four blocks compiled as given, and rustfmt, clippy and rustdoc left them
as they are. Its tests on the code before the story failed for US1-S1, US1-S3, US1-S4,
US1-S5, US1-S6, US1-S8, US1-S9 and US1-S10 and passed for US1-S2 and US1-S7; after the
story all passed three times, and both gates' Rust commands passed (1,453 tests under
nextest). Its diff was 14,792 B. It re-made the sixteen wrong variants then in the
table from their descriptions and saw the same scenarios fail in every row. Of fifteen
more of its own, two were bitten by no scenario, the last two rows above; the cases
that bite them are now in US1-S1 and US1-S2. It found no blocking defect, and its
seven defects of clarity (049-R1 to 049-R7) are folded here.

**Run on 2026-11-21**, on the branch `probe/049` at b7e983a0: a first probe test on
`main`'s code, in which `Orion Nebula`, `Great Orion Nebula`, `orion neb`,
`Ring Nebula`, `Whirlpool Galaxy`, `Double Cluster` and `North America Nebula` each
found nothing. `targets.rs` has not changed since.

**Computed from the extract** on 2026-11-22, unless dated, by a model of the rules
written apart from the code (the probe's notes):
- 182 common names on 152 of the 13,372 objects, counted once per row (175 distinct); 5
  hold a hyphen, 20 an apostrophe and 1 an `&`, and every character of every name is
  ASCII; 951 distinct prefixes of their whitespace words, 950 of them with a letter or a
  digit;
- with the rule, and no limit: each of the 950 finds at least the objects it found; the
  five of US1-S6 find more; `&` finds nothing;
- with the rule, at each limit from 1 to 50: from 5 on, none of the 950 loses an object
  it showed; at 1, `le`, `sn`, `sno` and `l'` show NGC 6705, NGC 6087, NGC 6087 and
  ESO 56-115 in place of UGC 5470, NGC 7662, NGC 7662 and NGC 6705; at 2 to 4, `l'`
  loses NGC 6705;
- 53 prefixes of the names' words read without their marks are not among the 951
  (`cats`, `bowtie`, `stephans`, `lecu`); each found nothing and now finds an object. A
  query that reads along a name past the end of a word (`catseye`, `orionn`) does too;
- how many names find their object when typed some way, with the rule:

  | Typed | Of | Found |
  |---|---|---|
  | As written, in capitals | the 182 names | 182 |
  | Marks left out, a hyphen as a space (`bow tie`, `cats eye`) | the 26 names with a mark | 26 |
  | A hyphen left out (`bowtie`) | the 5 names with a hyphen | 5 |
  | The first two words run together (`catseye`) | the 173 names of two or more words | 173 |
  | The words in reverse order | the same 173 | 173 |
  | One middle word left out | the 64 ways to do so in the 55 names of three or more words | 64 |
  | `'s` left off (`bode galaxy`) | the 19 names with one | 19 |
  | Each word cut to three letters, a hyphen typed as a space | the same 173 | 171 |
  | One word typed as two | 8 names tried | 8 |

  Words are those between spaces. The two of the last-but-one row not found are
  `bea paw gal` and `eig bur neb`, whose `paw` and `bur` follow a hyphen; cut the same
  way, the two hyphenated names of one word, `fou fig` and `wol lun mel`, find nothing
  either. The 8 names tried in the last row are Butterfly Cluster, Checkmark Nebula,
  Cocoon Nebula, Dumbbell Nebula, Fireworks Galaxy, Horsehead Nebula, Sunflower Galaxy
  and Whirlpool Galaxy, each with its first word typed as two (`horse head nebula`);
- none of the 13,372 canonical ids or the 110 Messier ids written with a space (13,482
  queries) matches any name, and none of the 651 duplicate aliases written with a space
  does; of the Caldwell numbers 1 to 109 written `C` and a space, four that are no ids
  of the catalog find a name: `C 30` NGC 2070, `C 3` NGC 2070 and NGC 1049, and `C 4`
  and `C 47` NGC 104;
- of the 14,462 distinct beginnings of the canonical ids and of the Messier ids written
  with a space, typed a character at a time, three that found nothing find a name:
  `cl 3` (NGC 2070 and NGC 1049), `c 4` (NGC 104) and `hc` (C 14); `cl 39` finds
  nothing;
- 33 canonical ids hold a mark other than the space, a hyphen, and are found with it
  only, as on `main` (`eso 56 115` finds nothing); `north american` finds NGC 7000;
- over the 1,999 beginnings of the 175 distinct names typed a character at a time, no
  added character adds a name match; reading along adds an object to what "each word
  begins a word" finds on two of them, `s n` and `s no`, typed toward `S Nor Cluster`,
  which also find NGC 7662 (`Copeland's Blue Snowball`);
- the rule first drafted for spec owner decision 2 (b), words split at hyphens and
  without `'` and `’`: 11 of the 951 found more, and at a limit of 20 `b` lost UGC 5373
  and `p` NGC 6357 (2026-11-21); a hyphen left out found 0 of 5, two words run together
  0 of 173, and one word typed as two 0 of 8;
- reading along alone (decision 2 (c)): the words in reverse order found 0 of 173, one
  middle word left out 2 of 64, and `'s` left off 0 of 19.

**Read, not run**:
- that the page sends the trimmed query as typed, a space as `+`
  (`crates/finder/web/src/api.ts:935`,
  `crates/finder/web/src/components/TargetSheet.tsx:57`), and that axum's `Query`
  decodes `+` as a space (US1-S8 sends `+` through the router, which is run);
- the cases of spec § Edge Cases that no scenario names, from the extract and the rule.

**Not run at all**: the page, `finder --simulate`, the page half of the `test` gate
(`npm ci` installs packages, and the story writes nothing the page is built from), and
the factory's gate boundary itself.

## Read from the diff, not from a test

- **The doc comment** of `targets::search` says the rule (FR-002).
- **Nothing else changes** (FR-003): the diff holds the three files of § Project
  Structure, and in `targets.rs` only the empty-query check, the filter, `words`,
  `name_matches`, the doc comment and the new tests. Both functions are private.

## Shapes that existing tests pin

Every existing test passes unchanged with the story built (run on the probe). The files
beside the change, and what the story does to what they observe:

- **`crates/finder/src/targets.rs`'s ten unit tests.**
  `search_uses_name_word_prefixes_and_orders_by_magnitude_with_unknowns_last`
  (`crates/finder/src/targets.rs:384`) searches single words only (`NEBULA`'s 60
  objects and their order, `cork`, `orion`, `romeda`, `qqq`, a blank query, a zero
  limit, `M31`, `ngc0651`); none of them is among the queries the rule changes
  (`NEBULA` still finds 60, NGC 3372 first, and `romeda` nothing, since reading along
  begins at a word): unchanged.
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
| Each query word begins a word of one name, any order | In order; next to each other only; across names; substring or fuzzy; distinct words | Spec owner decision 1. |
| In a common name only letters and digits are compared, and a query that reads along a name matches it | Marks left inside words; a list of marks, each split at or dropped; reading along alone | Spec owner decision 2. A list handles only the marks it names and changes more one-word answers; reading along alone loses decision 1 (a). |
| A name's words begin after spaces only | Also after hyphens | `bow tie` reads along `Bow-Tie` without it, and words after hyphens changed `b`, `l`, `m`, `p` and `t` (spec owner decision 2 (b)). |
| `str::to_lowercase` on the whole name or query, then `char::is_alphanumeric`, as the standard library has them | ASCII only; lowercasing character by character; a folding crate | Every name is ASCII today. The standard library keeps a letter outside ASCII a letter, which US1-S2's `orion é` holds for a query; no name has one, so that side is read from the code. The query reaches `words` lowercased as a whole, so names are lowercased the same way (the two ways differ only for `İ` and a final `Σ`; read, not tested). Folding accents is out of scope. |
| Ids are looked up as on `main` | Ids matched by letters and digits too | The lookup is 012's `normalize`, which FR-003 leaves alone; an id typed without its hyphen is out of scope. |
| A query with no word finds nothing, checked where the empty query is | Let `all` over no words match; look up the exact id first | `-` or `&` would answer every one of the 152 named objects (US1-S10, run as a wrong variant). Every id holds a letter or a digit, so none is lost. |
| Two private functions, `words` for names and queries alike and `name_matches` | The rule inline in the filter; words kept in `Target` at parse | Inline, the filter nests too deep to read once rustfmt has laid it out. A field would change the shape of the public `Target`, which § Technical Context's Constraints keep, and its JSON unless skipped; 182 names make their words per search cheap. |
| Change only `Catalog::search` | A second search for queries with a space; a change in `api::search` | One rule serves both doors and every query. |
| Unit tests in the module of 012's search tests, after its last test | A `console/` module for them | US1-S4 needs the private `Catalog::parse`, and the rest sit with the tests of the same function. |
| One `mcp/` module that asks the route and the tool | A `console/` module for the route and an `mcp/` module for the tool | The MCP app answers both, as `session_tools.rs` already compares them; one module, one `mod` line. |
| A new module | New cases in `session_tools.rs` or `console/targets.rs` | Existing tests are not edited (constitution II). |
| 047 in `depends_on_landed` | `concurrent` with 047's US3; dropping US1-S8 so the story need not wait (spec owner decision 3) | 047's US3 and this story each add a line to `crates/finder/tests/mcp/main.rs`. 047 has landed. |
| One story | Two | About 23 % of 65,536 B (§ Plan traps 2). |

## User Story 1 - A name of several words finds its object

Tests first: the nine unit tests in `targets.rs` and `tests/mcp/target_words.rs` with its
`mod` line. Then the empty-query check, the filter, `words`, `name_matches` and the doc
comment in `targets.rs`.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 28f436df):
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

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met at
   28f436df: 012, 016 and 027 before the draft, and 047 on 2026-11-22 (d3f8bd0f,
   c15937e7, 28f436df).
2. **`main` is still as read.** Every citation here was read at 28f436df on 2026-11-22,
   after 047 had landed. If `main` has since changed a file § Project Structure or
   § Shapes names, or `crates/finder/src/api.rs` or `crates/finder/web/src/api.ts`,
   re-read its citations and re-run the validator. Met at the flip: `main` was at
   75cb2c8c, moved since 28f436df only in documents no line here cites and by 048's two
   stories (its own trio, and the crates `star-detection`, `scope-node`, `simulator`
   and `plate-solver`), with nothing under `crates/finder` changed.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Decisions met: the owner answered 1 (a) and 3 (a) on
   2026-11-21, and chose decision 2's rule on 2026-11-22 ("Rewrite to this"); the trio
   is written for them. The approvals are given by the flip, which the owner approved
   on 2026-11-22 ("ok, approved. proceed.").
4. **Probe builds.** Met on 2026-11-22: probe round 1 built US1 from this trio alone as
   it stood at be52f8a0, before the round's findings were folded, with 0 blocking
   defects and 7 of clarity. The fold was made the same day; the test helper's signature
   and the two cases it added (US1-S1's tab, US1-S2's `bow bow tie`) are the trial
   build's, and pass there at a9211056 (§ What was run and what was read).

## Outside the factory

Written here and not run.

**A look at the page**, by the owner, once 049 has landed, on `finder --simulate`:

1. Open Choose target and type `Orion Nebula` in `Search catalog`: NGC 1976 (M42)
   should be the first row, and the only one.
2. Type `Ring Nebula`, `Whirlpool Galaxy` and `Double Cluster`: M57, M51 and C 14.
3. Type `cats eye`, `bowtie` and `horse head`: NGC 6543, NGC 40 and B 33.
4. Ask MCP `search_targets` with `{"q":"Great Orion Nebula"}`: NGC 1976.

**The voice design's C5.** Once 049 has landed, C5's row in
`docs/voice/02-tool-contracts.md:183` describes a change that is done, and its last
sentence and line 175's "only widens" no longer hold for the queries of spec owner
approval 2; marking and rewording them is the owner's.

**The Pi.** The console's binary changes, so a Pi kit built after 049 lands carries the
fix.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* the story's diff stays under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`, where `<base>` is
   the commit the story starts from, which already holds this trio. Measured on the
   drafting probe's net diff from 28f436df at a9211056: 14,832 B, 22.6 % of 65,536 B
   (`targets.rs` 12,345 B, the two `mcp/` files 2,487 B); probe round 1's build
   measured 14,792 B before its two cases were added. At the 1.35 times a factory build
   has run over its probe on earlier specs, about 20,000 B, 30.6 %.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/049-search-finds-whole-names/` or `.specify/`.
4. **An oracle copied from the code proves nothing.** *Check:* every expected list is a
   literal in the test, or computed in the test from `EXTRACT`'s text, with
   `targets::find` or `targets::all` naming the object a row or id stands for; no test
   calls `search`, `Catalog::search`, `words` or `name_matches` for an expected value
   or to choose its queries.
5. **A negative assertion passes when nothing happens.** US1-S2 and US1-S7 pass on
   `main`'s code. *Check:* US1-S1, US1-S3 to US1-S6, US1-S8 and US1-S9 assert objects
   that are found, and fail on `main`; US1-S10 fails on `main` by `&`.
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
| A new OpenNGC extract changes the counts and lists. | US1-S5 and US1-S6 assert their counts, 182, 951 and 950, so a new extract fails them by name; re-read the lists from it. |
| A later name holds a letter with an accent, which a keyboard may not type. | Such a letter is a word character and matches only itself (spec § Edge Cases); folding accents is out of scope, and no name has one today. |
| Reading along puts an object the observer did not mean first, such as `sn` finding NGC 6087 before NGC 7662. | Owner approval 2 names the change; US1-S6 lists every gain over today's 951 prefixes; over the 1,999 beginnings of the names typed in full, it adds an object on two (§ What was run and what was read). |
| An id typed with a space gains a name match on a later extract. | US1-S7 fails by that id; US1-S4 holds that the exact id still comes first. |

## Refinement Record

Drafted on 2026-11-21 by a plate-solver spec session against `main` at b7e983a0, and
rewritten on 2026-11-22 against `main` at 28f436df for the owner's choice of decision
2's rule, from a probe of the fault and a trial fix, a reading of
`crates/finder/src/targets.rs`, its callers in `api.rs` and `mcp/tools.rs`, their tests,
012's and 016's trios, the voice design's C5 and the constitution. Writes come from the
task text. Shared writes name the merge edge that orders them, and the story that last
wrote the same file on `main`.

The owner settled spec.md's three owner decisions: 1 (a) and 3 (a) on 2026-11-21, and
decision 2's rule on 2026-11-22. The record's `open_questions` are empty. This record
approves nothing: the owner approvals of spec.md are given by the flip.

```yaml
version: 1
reviewed_revision: 28f436dfbdd6b9e7aead37928f52a980027560fc
stories:
  US1:
    tests:
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "with a limit of 20, Orion Nebula, Great Orion Nebula, orion neb, ring NEBULA, Whirlpool Galaxy, Double Cluster, h & chi Persei, North America Nebula, spaced-out Ring Nebula, nebula orion, nebula then a tab then orion as one query, gr or ne and whirl gal give exactly NGC 1976, NGC 1976, NGC 1976, NGC 6720, NGC 5194, C 14, C 14, NGC 7000, NGC 6720, NGC 1976, NGC 1976, NGC 1976 and NGC 5194"
        decision: "spec.md:US1-S1"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "orion galaxy, andromeda nebula, barbell cork, rion nebula, orion nebulae, tie, tie nebula, orion with U+00E9 after a space and bow bow tie each find nothing"
        decision: "spec.md:US1-S2"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "cluster s gives Cl 399, NGC 6087, Mel 111; little nebula gives NGC 6818, NGC 650, NGC 6369, and with a limit of 2 NGC 6818, NGC 650; eagle nebula gives IC 4703, NGC 6611; orion o gives NGC 1976, IC 434"
        decision: "spec.md:US1-S3"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "on a parsed catalog of five rows where NGC 1 is M31 at magnitude 15, M 31 gives exactly NGC 1, NGC 2 (M 31 Bright Galaxy, 1), NGC 4 (Medium M31 Galaxy, 3), NGC 3 (Galaxy M 31, no magnitude), not NGC 5 (M 3 Galaxy)"
        decision: "spec.md:US1-S4"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "each of the extract's 182 common names counted per row, in capitals, with no limit, finds the object find gives for its row"
        decision: "spec.md:US1-S5"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "of the 951 distinct lowercased prefixes of the extract's common-name words, split at spaces and commas, 950 hold a letter or a digit; with no limit each of the 950 finds at least the rows with a common name having a whitespace-separated word that starts with it, and beyond those exactly: l' ESO 56-115, NGC 1980, IC 2944, NGC 6523, NGC 6618, NGC 6818, NGC 650, UGC 5470, NGC 6445, NGC 6369; l'e UGC 5470; le NGC 6705; sn and sno NGC 6087; nothing for the rest"
        decision: "spec.md:US1-S6"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "every canonical id, and every Messier id written M then a space then its number, finds exactly its object"
        decision: "spec.md:US1-S7"
      - path: crates/finder/tests/mcp/target_words.rs
        status: new
        disposition: add
        assertion: "on the console router, GET /api/targets with Orion+Nebula, Great+Orion+Nebula, Ring+Nebula, Whirlpool+Galaxy, Double+Cluster, Cat%E2%80%99s+Eye and Horse+Head answers 200 with exactly NGC 1976, NGC 1976, NGC 6720, NGC 5194, C 14, NGC 6543 and B 33; search_targets with each query is not an error, its structuredContent is targets set to the route's answer, and its one text block is JSON equal to it"
        decision: "spec.md:US1-S8"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "with a limit of 20, bow tie, bowtie and Bow-Tie give exactly NGC 40; cats eye, catseye, catseye nebula, cat's eye, cat with U+2019 s eye and cat eye nebula give exactly NGC 6543; bear paw NGC 2537; eight burst NGC 3132; Wolf Lundmark Melotte and Wolf-Lundmark-Melotte PGC 143; stephans quintet HCG 92; bode galaxy NGC 3031; horse head nebula B 33; whirl pool NGC 5194; jewelbox NGC 4755; h chi persei C 14"
        decision: "spec.md:US1-S9"
      - path: crates/finder/src/targets.rs
        status: new
        disposition: add
        assertion: "-, ', --, U+2019 alone, ' - and & each find nothing"
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
      - "a query of one word that holds a letter or a digit matches at least the objects it matched before, before the limit is applied; five of today's 951 one-word prefixes match more and & matches nothing (owner decision 2); the order is 012 FR-007's"
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
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US3
        disposition: merge_order
        reason: "049 depends_on_landed 047, whose US3 last changed mcp/main.rs (28f436df)"
    open_questions: []
```
