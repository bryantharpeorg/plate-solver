---
state: draft
depends_on_landed: ["012-targets-and-push-to", "016-mcp", "027-finder-tests-by-system", "042-imaging-run-page-and-mcp", "047-console-uses-the-database-that-fits"]
# Drafted 2026-11-21 against `main` at b7e983a0 (b7e983a031b6be86f894c1bbdff2ee3b04ba6cd3),
# from a fault seen that day while recording a README demo against `finder --simulate`,
# and a probe of the same day on the branch `probe/049` cut from b7e983a0: tests that
# show the fault on `main`'s code, and a trial fix with which they pass. Every
# `path:line` here was read at b7e983a0. The owner answered the three owner decisions on
# 2026-11-21 ("1. Agree 2. No hyphens or apostrophes require. 3. Ok"): 1 (a), 2 (b) and
# 3 (a); the trio is written for those answers, on a probe redone for 2 (b) the same day.
# It stays a draft: flipping it to `ready` is the owner's. 012 and 016
# are named because the story changes 012's search and adds a test beside 016's MCP
# tests; 027 because that test joins the `mcp` test binary 027 laid out; 042 because
# its US10 last changed `crates/finder/tests/mcp/main.rs`, which the story adds a line
# to; 047 because its US3 adds one there too, and 047 was building at b7e983a0. What
# rests on a run and what on reading is said where it is used (plan.md § What was run
# and what was read).
---

# Feature Specification: Target Search Finds Names of Several Words

**Feature Branch**: `049-search-finds-whole-names`

**Created**: 2026-11-21

**Status**: Draft

**Input**: On `finder --simulate`, `search_targets` with `{"q":"Orion Nebula"}` or
`{"q":"Great Orion Nebula"}` answers `[]`, while `"Orion"`, `"Nebula"` and `"M42"` each
find NGC 1976. A common name matches only when one of its words starts with the whole
query, so no query of two or more words finds any common name. The page's Target search
and the MCP tool share the search. Make a query of several words find the common name
that has them all, keep the order of the answers, and keep what every query of one word
finds. The owner added (owner decision 2): no one should need to type a name's hyphens
or apostrophes, which lets some queries of one word find more, and so, at a limit, show
another object in place of one they showed.

## Clarifications

### Session 2026-11-21 (the probe on `main` at b7e983a0)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interface, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it. The probe's branch
  and notes are not in the repository; what they showed is written here.
- Q: What does the search do today? → A: `Catalog::search`
  (`crates/finder/src/targets.rs:164`) lowercases and trims the query, takes an exact id
  if the query is one, and adds every object with a common name of which some word,
  split on whitespace, starts with the whole query
  (`crates/finder/src/targets.rs:181`). A query holding a space can start no word, so it
  finds no name. It then orders exact ids first, then by magnitude with no magnitude
  last, then by canonical id, and keeps `limit`. `targets::search`
  (`crates/finder/src/targets.rs:226`) calls it.
- Q: Who calls it? → A: `api::search` (`crates/finder/src/api.rs:1616`), which trims `q`,
  refuses an empty one with 400 and clamps `limit` to 1 to 50; the route
  `GET /api/targets` (`crates/finder/src/api.rs:612`, its handler at `:1608`) and the
  MCP tool `search_targets` (`crates/finder/src/mcp/tools.rs:615`) both answer through
  it. The page's Target sheet trims what is typed
  (`crates/finder/web/src/components/TargetSheet.tsx:57`) and sends it as `q`, a space
  as `+` (`crates/finder/web/src/api.ts:914`). So the fault and the fix reach the page
  and MCP alike, and only `crates/finder/src/targets.rs` changes.
- Q: What did the probe see? → A: On `main`'s code, the seven queries `Orion Nebula`,
  `Great Orion Nebula`, `orion neb`, `Ring Nebula`, `Whirlpool Galaxy`,
  `Double Cluster` and `North America Nebula` each found nothing; with the trial fix of
  plan.md § Interface, each finds its object alone (US1-S1). Every scenario's test was
  run on both, three times each (plan.md § What was run and what was read).
- Q: Which requirement of 012 does this change? → A: FR-007's second bullet. It reads:
  "then objects with a common-name word that the query begins (any case), brightest
  first, with objects that have no magnitude last"
  (`specs/012-targets-and-push-to/spec.md:640`). FR-001 here restates it for a query of
  several words; for a query of one word it says the same, except where a hyphen or an
  apostrophe, in the name or in the query, splits a word or is dropped (owner decision
  2). Its other clauses, exact ids first, the default `limit` of 20 clamped to 1 to 50,
  and the 400 for an empty `q`, are as they are, and so is the code's tie-break by
  canonical id among equal magnitudes (`crates/finder/src/targets.rs:194`), which FR-007
  does not state.
- Q: Was this planned before? → A: Yes. The voice-mode design lists it as console change
  C5 (`docs/voice/02-tool-contracts.md:183`): "A multi-word query should match when each
  of its words begins a word in one common name. Single-word results don't change".
  Owner decision 1's answer reads it so. Owner decision 2 departs from its last sentence
  where a hyphen or an apostrophe is in the name or the query.
- Q: Does a query of one word change? → A: Only through a hyphen or an apostrophe, in
  the name or in the query (owner decision 2). With one word, "each word begins a word"
  is today's rule, but a name's words are now also split at hyphens and lose their
  apostrophes, and so does the query: `l'` now finds what `l` finds. Over the 951
  prefixes of every word of the catalog's common names, as 012's rule splits them, every
  one finds at least the objects it found, and 11 find more (US1-S6). Of the 917
  prefixes of the new words, every name's words as owner decision 2 splits them, 44 that
  found nothing now find an object, such as `tie`, `paw`, `cats` and `melotte`
  (computed; of them only `tie` is tested, in US1-S9). At the default limit of 20, two
  of the 11 each lose one of their first 20 to the object they gain, which sorts ahead.
  `b` no longer shows UGC 5373, and `p` no longer shows NGC 6357. Other limits differ:
  at 10, `b`, `l`, `m` and `t` each lose one, and at 5, `bu` and `m`; from a limit of 28
  none loses one; and at every limit `me`, `le` and `l'` get a new first answer
  (computed, not tested).
- Q: Does an id typed with a space change? → A: No, on this catalog. `NGC 224` or `M 42`
  is now also read as two words, `ngc` and `224`, which a common name could hold; none
  of the catalog's 182 common names holds every word of any of its 13,372 canonical ids
  (all but M40 written with a space) or of its 110 Messier ids written with one, so each
  still finds its object alone (US1-S7). The exact id still comes first if one ever did
  (US1-S4). A designation with a space that is no id of the catalog can now find a name
  (§ Edge Cases).
- Q: Why 047 in `depends_on_landed`? → A: 047's US3 adds `mod databases;` to
  `crates/finder/tests/mcp/main.rs` (047's plan.md § Refinement Record), and this story
  adds `mod target_words;` to the same file, after `mod steering;`
  (`crates/finder/tests/mcp/main.rs:20`), where 047's goes after `mod calibration;`
  (`:8`). The lines are not neighbours, so git would merge the two either way; waiting
  keeps two epics from writing one file at once while 047 is building (owner decision
  3). 047's US1 landed during the draft (d3f8bd0f); its US2 and US3 had not. The story
  writes no other file 047 writes; it cites `crates/finder/src/api.rs` and
  `crates/finder/web/src/api.ts`, which 047 changes (plan.md § Prerequisites 2).
- Q: What rests on a run, and what on reading? → A: Every scenario was run on the probe
  three times on `main`'s search code and three times with the trial fix; plan.md
  § What was run and what was read lists each claim. The page and `finder --simulate`
  were not run.
- Q: What is left for later? → A: The items under "Out of scope" in § Assumptions.

## Owner decisions

The owner answered all three on 2026-11-21 ("1. Agree 2. No hyphens or apostrophes
require. 3. Ok"): the recommended answers to 1 and 3, and (b) to 2. The scenarios are
written for those answers; the alternatives are kept below for the record.

1. **The rule for a query of several words** (FR-001; US1-S1 to US1-S3, US1-S8).
   *Settled 2026-11-21: the recommended answer (a).* Recommended: (a) a common name
   matches when each word of the query begins one of the name's words, in any order, and
   two query words may begin the same name word. It is one reading of C5, which does not
   say whether they may; (f) is the other. `orion neb`, `nebula orion` and `gr or ne`
   find NGC 1976; `orion o` finds NGC 1976 and IC 434 (`Orion B`); `cluster s` finds the
   three clusters whose names hold `cluster` and a word starting `s` (`S Nor Cluster`,
   `Al Sufi's Cluster`, `Coma Star Cluster`). Weighed against:
   - **(b) the words in the name's order**, each query word beginning a later name
     word than the one before, not necessarily next to it: the same answers for every
     common name typed as it is written; `nebula orion`, `cluster s` and `orion o`
     find nothing. A spoken or remembered name in another order is lost,
     for little gain on this catalog: of the 312 queries made of two of a common name's
     words in that name's order (285 distinct), (a) and (b) answer alike for all but
     one, and for `cluster 3`
     (a) adds NGC 2070 (`30 Dor Cluster`) to NGC 1049 (`Fornax Dwarf Cluster 3`)
     (plan.md § What was run and what was read).
   - **(c) the words next to each other in the name's order**, as a phrase: as (b),
     and `great nebula` no longer finds the Great Orion Nebula.
   - **(d) the words from any of an object's common names**: `barbell cork` would find
     M76 by its two names `Barbell Nebula` and `Cork Nebula`. No one types that, and it
     widens queries of several words: `cluster s` would add NGC 6705, from
     `Amas de l'Ecu de Sobieski` and `Wild Duck Cluster`.
   - **(e) substring or fuzzy matching**: `romeda` would find M31, which 012's test
     holds must find nothing (`crates/finder/src/targets.rs:416`), so (e) would need an
     existing test edited, which the constitution's Principle II forbids without the
     owner's approval; and a query of one word would change.
   - **(f) each query word beginning a different word of the name**: stricter where a
     query reuses a name word. `orion o` finds nothing, where (a) finds NGC 1976 and
     IC 434 (US1-S3). While a name is typed the two almost never differ: of the 1,999
     distinct lowercased prefixes of the 175 distinct names, typed a character at a
     time, they differ on 3, `chi persei c`, `coalsack c` and `small s`, and on each (a)
     finds one more object (C 14, C 99 and NGC 292) beside the one being typed; for
     `small s`, NGC 292 (magnitude 2.3) comes before IC 4715 (4.5), the Small Sgr Star
     Cloud being typed. Every full name is answered alike. (a) is recommended as the
     simpler rule: (f) needs a matching of query words to name words for those few
     differences.
2. **Hyphens and apostrophes in names** (FR-001; US1-S6, US1-S9, US1-S10). *Settled
   2026-11-21: (b), the owner's answer, "No hyphens or apostrophes require."* (b): names
   and queries alike are split at hyphens as well as whitespace, and lose their
   apostrophes, the straight `'` and the curly `’` (U+2019) that phone keyboards type.
   The curly `’`, and the rule that a query with no word finds nothing, are the author's
   reading of (b), which the flip approves. `bow tie`, `Bow-Tie` and `tie` find NGC 40
   (`Bow-Tie nebula`); `cats eye`, `cat's eye` and `cat’s eye` find NGC 6543
   (`Cat's Eye Nebula`). Of the 182 common names, 5 hold a hyphen (`Bear-Paw`,
   `Bow-Tie`, `Eight-Burst`, `Fourcade-Figueroa`, `Wolf-Lundmark-Melotte`) and 20 an
   apostrophe. A query of one word changes with it: every one of today's 951 one-word
   prefixes finds at least the objects it found, 11 of them (among them `b`, `m`, `p`
   and `t`) find more (US1-S6), and 44 of the 917 prefixes of the new words, which found
   nothing, now find an object (computed; only `tie` is tested, in US1-S9). A query's
   own apostrophe is dropped too, so `l'` finds what `l` finds. At the default limit of
   20, two of the 11 each lose one of their first 20 to the object they gain, which
   sorts ahead. `b` no longer shows UGC 5373, and `p` no longer shows NGC 6357. Other
   limits differ: at 10, `b`, `l`, `m` and `t` each lose one, and at 5, `bu` and `m`;
   from a limit of 28 none loses one; and at every limit `me`, `le` and `l'` get a new
   first answer (computed, not tested). A query left with no word, such as `-` or `'`,
   finds nothing, as today (US1-S10). The alternatives, kept for the record: (a), the
   draft's recommendation, left them inside words, so `bow tie` and `cats eye` found
   nothing and no one-word answer changed; splitting names but not queries would lose 25
   of the 182 names typed in full, `Bow-Tie nebula` among them.
3. **When the story may run** (front matter; plan.md § Refinement Record). *Settled
   2026-11-21: the recommended answer (a).* Recommended: (a) after 047 has landed: 047
   stays in `depends_on_landed`, and the shared write of
   `crates/finder/tests/mcp/main.rs` is `merge_order`. 047 was building at b7e983a0 and
   its US1 landed during the draft; the owner's decisions 1 and 2, and a probe round,
   come first in any case (plan.md § Prerequisites). No two epics write one file at
   once, and the re-read of § Prerequisites 2 is done once. Weighed against: (b)
   `concurrent` with 047's US3: 047 leaves `depends_on_landed`, and the record's entry
   for `mcp/main.rs` and 047's US3 becomes `concurrent` with its reason, which the
   validator takes as advisory only ("cross-spec concurrency is advisory only and must
   be resolved by final release ownership"); the two `mod` lines are not neighbours, so
   git merges them, but the story could run while 047's US3 is building. Under (b) the
   passages that say 049 waits for 047 are rewritten too: the front-matter comment, the
   Q "Why 047", plan.md § Prerequisites 1 and 2 and tasks.md's prerequisites. Or (c)
   drop US1-S8 and `crates/finder/tests/mcp/`, so the story writes only `targets.rs` and
   need not wait. No test would then send a query of several words through the route and
   the tool; 016's `crates/finder/tests/mcp/session_tools.rs:59` holds the tool equal to
   the route for single words only.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **The search finds more than it did** (FR-001), in place of 012's FR-007 second
   bullet for a query of several words (`specs/012-targets-and-push-to/spec.md:640`): a
   query of several words that found nothing now finds the objects whose common name
   holds every word. No canonical id and no Messier id written with a space finds
   anything else on this catalog (US1-S7). A designation with a space that is no id of
   the catalog can now find names: `C 30` finds NGC 2070 (`30 Dor Cluster`), and `C 3`
   finds NGC 2070 and NGC 1049 (§ Edge Cases). An observer who types a Caldwell number
   that way gets a wrong object first where today they get nothing. `GET /api/targets`
   and `search_targets` keep their shapes.
2. **A query of one word finds more where it or a name has a hyphen or an apostrophe**
   (FR-001, owner decision 2), in place of 012's FR-007 second bullet for those words:
   11 of today's 951 one-word prefixes find the objects US1-S6 lists beside what they
   found, `l'` among them, which now finds what `l` finds; and 44 of the 917 prefixes of
   the new words, which found nothing, find an object (`tie` finds NGC 40, as US1-S9
   tests; the rest computed, not tested). No one-word query stops matching an object it
   matched (US1-S6), but a gain that sorts ahead takes a place in the answer. At the
   default limit of 20, `b` and `p` each lose one of their first 20 to the object they
   gain: `b` no longer shows UGC 5373, and `p` no longer shows NGC 6357. Other limits
   differ: at 10, `b`, `l`, `m` and `t` each lose one, and at 5, `bu` and `m`; from a
   limit of 28 none loses one; and at every limit `me`, `le` and `l'` get a new first
   answer (computed, not tested).

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command.
- **The catalog** is the pinned OpenNGC extract `crates/finder/data/openngc.csv`, as
  compiled into the binary, and **the extract** is that file's text. An object is named
  by its canonical id (`NGC 1976`, `C 14`, `Mel 111`). **A search** is
  `targets::search(query, limit)`, or, in US1-S4, `Catalog::search` on the catalog the
  test parses, and **its ids** are its objects' canonical ids, in order.
- **The console router** is `support::mcp::app(support::mcp::state(), "127.0.0.1")`, as
  `crates/finder/tests/mcp/session_tools.rs:60` builds it: the router in process, with
  the vendored cedar database and the fixed clock. **The route's answer** is
  `GET /api/targets?q=<query>`'s JSON, with each space of the query sent as `+`, as the
  page sends it. **The tool's answer** is the JSON-RPC `result` of `tools/call` for
  `search_targets` with `{"q": <query>}`.
- **Oracles.** The extract's `name`, `mag`, `messier` and `common_names` columns, read
  by the test or written in it from the extract; 012's single-word rule as FR-007
  words it, applied by the test to the extract's rows; an independent catalog the test
  writes in the extract's format; the route's own answer for the tool's; and the words
  in these scenarios. `targets::find` and `targets::all`, which FR-003 leaves unchanged,
  may name the object a row or id stands for. No expected list comes from the code
  under test: no test calls `search` for an expected value.

### User Story 1 - A name of several words finds its object (Priority: P1)

As the observer, I type "Orion Nebula", or "Ring Nebula", or "Whirlpool Galaxy", in the
console's Target search or through MCP, and the object comes back first, as it does for
"Orion" or "M42".

**Why this priority**: The names observers know best are two or three words long, and
every one of them, typed in full, finds nothing today.

**Independent Test**: the unit tests of `crates/finder/src/targets.rs` (US1-S1 to
US1-S7, US1-S9 and US1-S10) and the new module
`crates/finder/tests/mcp/target_words.rs` (US1-S8). The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs them; during
development, `cargo nextest run -p finder --locked --lib targets::` and
`cargo nextest run -p finder --locked --test mcp target_words::`.

**Acceptance Scenarios**:

1. **Given** the catalog, **When** a committed test searches with a limit of 20 for
   each of `Orion Nebula`, `Great Orion Nebula`, `orion neb`, `ring NEBULA`,
   `Whirlpool Galaxy`, `Double Cluster`, `h & chi Persei`, `North America Nebula`,
   `  Ring   Nebula `, `nebula orion`, `gr or ne` and `whirl gal`, **Then** it asserts
   that their ids are, in turn, exactly `[NGC 1976]`, `[NGC 1976]`, `[NGC 1976]`,
   `[NGC 6720]`, `[NGC 5194]`, `[C 14]`, `[C 14]`, `[NGC 7000]`, `[NGC 6720]`,
   `[NGC 1976]`, `[NGC 1976]` and `[NGC 5194]`.
2. **Given** the catalog, **When** a committed test searches with a limit of 20 for each
   of `orion galaxy`, `andromeda nebula`, `barbell cork`, `rion nebula` and
   `orion nebulae`, **Then** it asserts that each finds nothing.
3. **Given** the catalog, **When** a committed test searches for `cluster s` and
   `little nebula` with a limit of 20, for `little nebula` with a limit of 2, and for
   `eagle nebula` and `orion o` with a limit of 20, **Then** it asserts that their ids
   are exactly `[Cl 399, NGC 6087, Mel 111]`, `[NGC 6818, NGC 650, NGC 6369]`,
   `[NGC 6818, NGC 650]`, `[IC 4703, NGC 6611]` and `[NGC 1976, IC 434]`.
4. **Given** a catalog the test parses with `Catalog::parse` from the extract's header
   and five rows, `NGC0001;G;10;20;And;;;;15;M31;Faint Galaxy;`,
   `NGC0002;G;10;20;And;;;;1;;M 31 Bright Galaxy;`,
   `NGC0003;G;10;20;And;;;;;;Galaxy M 31;`,
   `NGC0004;G;10;20;And;;;;3;;Medium M31 Galaxy;` and
   `NGC0005;G;10;20;And;;;;2;;M 3 Galaxy;`, **When** a committed test searches it for
   `M 31` with a limit of 20, **Then** it asserts that the ids are exactly
   `[NGC 1, NGC 2, NGC 3]`.
5. **Given** every common name of every row of the extract that is neither `Dup` nor
   `NonEx`, **When** a committed test searches for each name in capitals with no limit
   (`usize::MAX`), **Then** it asserts that each search holds the object `find` gives
   for that row's `name` column, and that there are 182 names, counted once per row they
   appear in (175 of them distinct, which is not asserted).
6. **Given** every prefix of every word of those common names, lowercased, a word being
   what lies between spaces and commas as 012's rule reads them, **When** a committed
   test searches for each with no limit, **Then** it asserts that there are 951 distinct
   such queries; that each one's ids, as a set, hold the canonical ids of the rows
   having a common name with a whitespace-separated word, as 012's rule reads it, that
   starts with the query, compared in lowercase; and that the ids beyond those are
   exactly these, and none for any other query: for `b` and `bu`, NGC 3132; for `fi`,
   ESO 270-017; for `l`, `m` and `me`, PGC 143; for `l'`, ESO 56-115, NGC 1980, IC 2944,
   NGC 6523, NGC 6618, NGC 6818, NGC 650, UGC 5470, PGC 143, NGC 6445 and NGC 6369; for
   `l'e`, UGC 5470; for `le`, NGC 6705; for `p`, NGC 2537; for `t`, NGC 40.
7. **Given** every object of `targets::all()`, **When** a committed test searches with no
   limit for its canonical id and, for each object with a Messier id, for that id with a
   space after the `M` (`M 42`), **Then** it asserts that each search's ids are exactly
   that object's id.
8. **Given** the console router, **When** a committed test asks the route and the tool for
   each of `Orion Nebula`, `Great Orion Nebula`, `Ring Nebula`, `Whirlpool Galaxy`,
   `Double Cluster` and `Cat’s Eye` (with U+2019, sent to the route as
   `Cat%E2%80%99s+Eye`), **Then** it asserts that the route answers 200 with a list
   whose ids are, in turn, exactly `[NGC 1976]`, `[NGC 1976]`, `[NGC 6720]`,
   `[NGC 5194]`, `[C 14]` and `[NGC 6543]`; that the tool's answer is not an error; that
   its `structuredContent` is `{"targets": <the route's answer>}`; and that it has one
   content block, whose text is JSON equal to the route's answer.
9. **Given** the catalog, **When** a committed test searches with a limit of 20 for each
   of `bow tie`, `Bow-Tie`, `tie`, `cats eye`, `cat's eye`, `cat’s eye` (with U+2019),
   `bear paw`, `eight burst`, `Wolf Lundmark Melotte`, `Wolf-Lundmark-Melotte` and
   `stephans quintet`, **Then** it asserts that their ids are, in turn, exactly
   `[NGC 40]`, `[NGC 40]`, `[NGC 40]`, `[NGC 6543]`, `[NGC 6543]`, `[NGC 6543]`,
   `[NGC 2537]`, `[NGC 3132]`, `[PGC 143]`, `[PGC 143]` and `[HCG 92]`.
10. **Given** the catalog, **When** a committed test searches with a limit of 20 for each
    of `-`, `'`, `--`, `’` (U+2019) and `' -`, **Then** it asserts that each finds
    nothing.

### Edge Cases

- **A query of one word**, one whitespace-separated piece as 012's rule reads it,
  matches at least the objects it matched before. Without a hyphen or an apostrophe it
  is one query word, so "each query word" is today's word. No match is lost on this
  catalog, where no name word begins with a hyphen or an apostrophe: a query that began
  a piece of a name, split and stripped as the name is, begins the name's new words.
  Only a hyphen or an apostrophe, in the name or in the query, can add a match (`l'`
  finds what `l` finds), and the sort is untouched. US1-S6 checks the objects as sets;
  012's tests of `crates/finder/src/targets.rs:384` and
  `crates/finder/tests/console/targets.rs:72`, unchanged, check the first result of the
  queries they name and the brightness order of `NEBULA`'s, and pass with the fix. A
  gain can push a match past `limit`: at 20, `b` and `p` each lose one of their first 20
  to it (owner approval 2; computed, not tested).
- **Spaces.** The query is trimmed and split at whitespace and hyphens, so leading,
  trailing and repeated spaces do not matter (US1-S1's `  Ring   Nebula `). An empty or
  blank `q` is still refused with 400 before the search
  (`crates/finder/src/api.rs:1622`).
- **Two query words may begin the same name word**: `orion o` finds NGC 1976 and then
  IC 434 (`Orion B`) (owner decision 1 (a); US1-S3).
- **Words from two names of one object** do not combine: `barbell cork` finds nothing,
  though M76 is both the Barbell Nebula and the Cork Nebula (US1-S2).
- **Hyphens and apostrophes** need not be typed, and may be (owner decision 2; US1-S9):
  a query is split and stripped as names are, so `bow-tie` and `cat's eye` still find
  their objects, and `cat eye` still finds NGC 6543. Only the ASCII hyphen `-` splits,
  and only `'` and `’` are dropped: an en dash or another mark stays part of a word.
  `l'Ecu` becomes `lecu`, so `lecu` and `l'ecu` find NGC 6705, and `ecu`, as today, does
  not. Read from the rule, not tested, past US1-S9.
- **A query of only hyphens, apostrophes and spaces** has no word and finds no name
  (US1-S10); without that guard every named object would match.
- **A word freed by a split can surprise**: `mel`, typed toward `Mel 22`, now finds PGC
  143 (`Wolf-Lundmark-Melotte`); `mel 2` finds nothing, and `mel 22` finds Mel 22. `mel`
  is no prefix of today's words, so US1-S6 does not list it. Computed, not tested.
- **Equal magnitudes** are ordered by canonical id, as today: `eagle nebula` gives
  IC 4703 before NGC 6611, which is M16 (US1-S3). Both carry the name in OpenNGC.
- **An id typed with a space whose words a name also holds** would put the exact id
  first and then the names by brightness (US1-S4). No such id is in the catalog
  (US1-S7).
- **A designation typed with a space that is not a catalog id** can now find names
  holding its words: `C 30` finds NGC 2070 (`30 Dor Cluster`), and `C 3` finds NGC 2070
  and NGC 1049 (`Fornax Dwarf Cluster 3`), where both find nothing today. The catalog
  holds Caldwell ids only for objects without an NGC or IC id (`C 9`, `C 14`, `C 41`,
  `C 99`); Caldwell 30 and Caldwell 3 are NGC 7331 and NGC 4236 (by reading, not from the
  extract), which the catalog holds under those ids alone, so `C 30` gives a wrong object
  first. None of the extract's 651 duplicate aliases, written with a space, finds a
  name. Read from the extract and the rule, not tested.
- **The page** sends what is typed as it is typed, trimmed, after each keystroke
  (`crates/finder/web/src/components/TargetSheet.tsx:57`). As a name is typed, a
  query of several words keeps narrowing its name matches: each added word can only
  remove them. An exact id can still appear as words are added (`ngc` finds nothing,
  `ngc 1` finds NGC 1), as on `main`. Read from the rule, not run on the page.

## Requirements *(mandatory)*

### Functional Requirements

**The search** (US1):

- **FR-001**: `targets::search` SHALL answer, after any exact id, the objects with a
  common name in which each word of the query begins some word of the name, in any order
  (owner decision 1). The words of a query or a name SHALL be its pieces once trimmed,
  lowercased, freed of every `'` (U+0027) and `’` (U+2019), and split at whitespace and
  at `-` (U+002D), with empty pieces dropped (owner decision 2). A query with no word
  SHALL find no name. The exact id SHALL still be looked up from the trimmed, lowercased
  query as today. The order SHALL stay as it is: exact ids first, then by magnitude with
  no magnitude last (012's FR-007), then by canonical id
  (`crates/finder/src/targets.rs:194`); at most `limit` objects, with no duplicates. A
  query of one whitespace-separated piece that has a word SHALL match every object 012's
  rule matches for it, before the limit is applied.
- **FR-002**: The doc comment of `targets::search` SHALL state the rule of FR-001: exact
  ids first, then common names in which each word of the query begins a word, in any
  order, with words split at spaces and hyphens, apostrophes and case ignored, in
  brightness order.
- **FR-003**: Nothing else SHALL change. The story's changes are FR-001's filter, a
  private function that makes the words, and FR-002's doc comment in
  `crates/finder/src/targets.rs`, new tests after that module's
  last test, `every_source_object_is_reachable_by_its_raw_and_canonical_id`
  (`crates/finder/src/targets.rs:454`), the new module
  `crates/finder/tests/mcp/target_words.rs`, and its one `mod` line in
  `crates/finder/tests/mcp/main.rs`. `targets::find`, `targets::all`, the catalog's
  parsing and data, `crates/finder/src/api.rs`, `crates/finder/src/mcp/`, the page,
  `crates/finder/web/dist`, every other file under `crates/`, and every existing test
  and fixture are unchanged.

### Key Entities

- **A query word**: a piece of the trimmed, lowercased query without its apostrophes,
  split at whitespace and hyphens.
- **A name word**: a piece of one lowercased common name, as OpenNGC gives the name,
  without its apostrophes, split at whitespace and hyphens.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited. Every existing
  test passes unchanged (plan.md § Shapes that existing tests pin).
- **SC-003**: Every common name of the catalog, typed in full, finds its object
  (US1-S5), where today every name of two or more words finds nothing.
- **SC-004**: Every query of one word matches at least the objects it matched before,
  and more only where the name or the query holds a hyphen or an apostrophe (US1-S6);
  every canonical id and every Messier id written with a space finds its object alone
  (US1-S7).
- **SC-005**: Each name of US1-S9 is found without its hyphens or apostrophes (US1-S9),
  and with them (US1-S5 and US1-S9).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed.
  Every citation here was read at b7e983a0.
- **The catalog is the pinned extract.** The counts 182 and 951 and every expected list
  are read from `crates/finder/data/openngc.csv` at b7e983a0; a new extract would need
  them read again.
- **Out of scope:**
  - dashes other than `-` and apostrophe-like marks other than `'` and `’`;
  - ranking by how well a name matches, or Messier objects first on equal magnitude
    (`eagle nebula` keeps IC 4703 before M16);
  - a search of names of two words written as one (`northamerica`), misspellings, and
    star names, the Moon and planets, which 012 left out;
  - marking C5 done in `docs/voice/02-tool-contracts.md`, which is the owner's design
    note (plan.md § Outside the factory).
- **Outside the factory:** a look at the page's Target search on `finder --simulate`,
  in plan.md § Outside the factory, by the owner. The console's binary changes, so a
  Pi kit built after 049 lands carries the fix.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's jobs run on the pull request and in the merge queue, outside the
  gates the judge sees, so no scenario's Then clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003]
```

One story: `Catalog::search`'s filter, a private function for its words,
`targets::search`'s doc comment, and their tests (plan.md § Plan traps 2).
