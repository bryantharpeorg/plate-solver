---
state: draft
depends_on_landed: ["012-targets-and-push-to", "016-mcp", "027-finder-tests-by-system", "047-console-uses-the-database-that-fits"]
# Drafted 2026-11-21 against `main` at b7e983a0, from a fault seen that day while
# recording a README demo against `finder --simulate`, and re-read on 2026-11-22 against
# `main` at 28f436df (28f436dfbdd6b9e7aead37928f52a980027560fc), once 047 had landed.
# Every `path:line` here was read at 28f436df. A probe of 2026-11-22 on the branch
# `probe/049-letters`, rebased on 28f436df, holds tests that show the fault on `main`'s
# code and a trial fix with which they pass. The owner answered the three owner
# decisions on 2026-11-21 ("1. Agree 2. No hyphens or apostrophes require. 3. Ok") and,
# on 2026-11-22, asked for a simpler rule for decision 2 than the draft's first reading
# of his answer, and chose the one written here ("Rewrite to this"). It stays a draft:
# flipping it to `ready` is the owner's. 012 and 016 are named because the story
# changes 012's search and adds a test beside 016's MCP tests; 027 because that test
# joins the `mcp` test binary 027 laid out; 047 because its US3 last changed
# `crates/finder/tests/mcp/main.rs`, which the story adds a line to. What rests on a run
# and what on reading is said where it is used (plan.md § What was run and what was
# read).
---

# Feature Specification: Target Search Finds Names of Several Words

**Feature Branch**: `049-search-finds-whole-names`

**Created**: 2026-11-21

**Status**: Draft

**Input**: On `finder --simulate`, `search_targets` with `{"q":"Orion Nebula"}` or
`{"q":"Great Orion Nebula"}` answers `[]`, while `"Orion"`, `"Nebula"` and `"M42"` each
find NGC 1976. A common name matches only when one of its words starts with the whole
query, so no query of two or more words finds any common name. The page's
`Search catalog` field and the MCP tool share the search. Make a query of several words
find the common name that has them all, and keep the order of the answers. The owner
added (owner decision 2): no one should need to type a name's hyphens or apostrophes.
The rule he chose for that compares only the letters and digits of a name and of the
query, so no mark and no space has to be typed as the name has it.

## Clarifications

### Session 2026-11-22 (the probe on `main` at 28f436df)

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
- Q: Who calls it? → A: `api::search` (`crates/finder/src/api.rs:1652`), which trims `q`,
  refuses an empty one with 400 and clamps `limit` to 1 to 50; the route
  `GET /api/targets` (`crates/finder/src/api.rs:620`, its handler at `:1644`) and the
  MCP tool `search_targets` (`crates/finder/src/mcp/tools.rs:615`) both answer through
  it. The page's Choose a target dialog trims what is typed in `Search catalog`
  (`crates/finder/web/src/components/TargetSheet.tsx:57`) and sends it as `q`, a space
  as `+` (`crates/finder/web/src/api.ts:935`). So the fault and the fix reach the page
  and MCP alike, and only `crates/finder/src/targets.rs` changes.
- Q: What are the two ways a name matches? → A: Both compare **words**: the pieces of a
  query or of a name between spaces, lowercased and each kept to its letters and digits
  (`Cat's` is `cats`, `Bow-Tie` is `bowtie`, `&` is no word). **Each word begins a
  word** when every word of the query begins some word of the name, in any order
  (owner decision 1). The query **reads along** the name when the query's words, written
  with nothing between them, begin the name's words written the same way from one of
  the name's words on (owner decision 2): `bow tie` and `horse head` read along
  `Bow-Tie nebula` and `Horsehead Nebula`, and `orion neb` reads along
  `Great Orion Nebula` from its second word. A name matches when either holds (FR-001).
- Q: What did the probe see? → A: On `main`'s code, the seven queries `Orion Nebula`,
  `Great Orion Nebula`, `orion neb`, `Ring Nebula`, `Whirlpool Galaxy`,
  `Double Cluster` and `North America Nebula` each found nothing (run on 2026-11-21 at
  b7e983a0; `targets.rs` has not changed since); with the trial fix of plan.md
  § Interface, each finds its object alone (US1-S1). Every scenario's test was run on
  both, three times each, at 28f436df; the two cases added last, US1-S1's tab and
  US1-S2's `bow bow tie`, were run on `main`'s code once (plan.md § What was run and
  what was read).
- Q: Which requirement of 012 does this change? → A: FR-007's second bullet. It reads:
  "then objects with a common-name word that the query begins (any case), brightest
  first, with objects that have no magnitude last"
  (`specs/012-targets-and-push-to/spec.md:640`). FR-001 here replaces how a name
  matches. Its other clauses, exact ids first, the default `limit` of 20 clamped to 1 to
  50, and the 400 for an empty `q`, are as they are, and so is the code's tie-break by
  canonical id among equal magnitudes (`crates/finder/src/targets.rs:194`), which FR-007
  does not state.
- Q: Was this planned before? → A: Yes. The voice-mode design lists it as console change
  C5 (`docs/voice/02-tool-contracts.md:183`): "A multi-word query should match when each
  of its words begins a word in one common name. Single-word results don't change".
  Owner decision 1's answer reads it so. Owner decision 2 adds reading along, and
  departs from C5's last sentence for the six one-word queries of the next answer that
  find something today, and for those that found nothing and now find an object; for
  one of the six, `&`, it also departs from that section's line that C5 "only widens
  what a search can find" (`docs/voice/02-tool-contracts.md:175`).
- Q: Does a query of one word change? → A: Rarely. A query of one word that holds a
  letter or a digit finds at least what it found (FR-001). Over the 951 prefixes of
  every word of the catalog's common names, as 012's rule splits them, 950 hold a letter
  or a digit, and five of those find more (US1-S6):
  - `le` gains NGC 6705, whose name `Amas de l'Ecu de Sobieski` has the word `lecu`;
  - `sn` and `sno` gain NGC 6087, since they read along `S Nor Cluster`;
  - `l'` finds what `l` finds, and `l'e` what `le` finds.

  `&`, the one prefix with no letter or digit, finds nothing, where it found C 14
  (US1-S10). A query of one word that found nothing can now find an object where it
  passes over a mark or a space: `bowtie` and `catseye` (US1-S9), `cats` and `stephans`
  (computed). No one-letter query changes. At a limit of 5 or more, none of the 950
  loses an object it showed; at a limit of 1, `le`, `sn`, `sno` and `l'` show the
  object they gain, which is brighter, in place of the one they showed; at 2 to 4, `l'`
  no longer shows NGC 6705, the fifth of its eleven (computed, not tested).
- Q: Does an id typed with a space change? → A: No, on this catalog. `NGC 224` or `M 42`
  is now also read as two words, `ngc` and `224`, and along a name as `ngc224`; none of
  the catalog's 182 common names matches any of its 13,372 canonical ids (all but M40
  written with a space) or of its 110 Messier ids written with one, so each still finds
  its object alone (US1-S7). The exact id still comes first if one ever did (US1-S4). A
  designation with a space that is no id of the catalog can now find a name (§ Edge
  Cases).
- Q: Why 047 in `depends_on_landed`? → A: Its US3 last changed
  `crates/finder/tests/mcp/main.rs` (28f436df), adding `mod databases;`
  (`crates/finder/tests/mcp/main.rs:9`), and this story adds `mod target_words;` to the
  same file, after `mod steering;` (`crates/finder/tests/mcp/main.rs:21`). 047's three
  stories landed by 2026-11-22 (d3f8bd0f, c15937e7, 28f436df), so the wait of owner
  decision 3 is over.
- Q: What rests on a run, and what on reading? → A: Every scenario was run on the probe
  three times on `main`'s search code and three times with the trial fix, except that
  the two cases added last (US1-S1's tab and US1-S2's `bow bow tie`) were run on
  `main`'s search code once; plan.md § What was run and what was read lists each claim.
  The page and `finder --simulate` were not run.
- Q: What is left for later? → A: The items under "Out of scope" in § Assumptions.

## Owner decisions

The owner answered all three on 2026-11-21 ("1. Agree 2. No hyphens or apostrophes
require. 3. Ok"). On 2026-11-22 he found the draft's first reading of his answer to 2
too complicated, asked for a simpler rule, and chose the one below ("Rewrite to this").
The scenarios are written for these answers; the alternatives are kept for the record.

1. **The rule for a query of several words** (FR-001; US1-S1 to US1-S3, US1-S8).
   *Settled 2026-11-21: the recommended answer (a).* (a): a common name matches when
   each word of the query begins one of the name's words, in any order, and two query
   words may begin the same name word. It is one reading of C5, which does not say
   whether they may; (f) is the other. `orion neb`, `nebula orion` and `gr or ne` find
   NGC 1976; `orion o` finds NGC 1976 and IC 434 (`Orion B`); `cluster s` finds the
   three clusters whose names hold `cluster` and a word starting `s` (`S Nor Cluster`,
   `Al Sufi's Cluster`, `Coma Star Cluster`). Weighed against:
   - **(b) the words in the name's order**, not necessarily next to each other:
     `nebula orion`, `cluster s` and `orion o` find nothing. A spoken or remembered
     name in another order is lost.
   - **(c) the words next to each other in the name's order only**, as a phrase: as
     (b), and `great nebula` no longer finds the Great Orion Nebula.
   - **(d) the words from any of an object's common names**: `barbell cork` would find
     M76 by its two names `Barbell Nebula` and `Cork Nebula`. No one types that, and it
     widens queries of several words.
   - **(e) substring or fuzzy matching**: `romeda` would find M31, which 012's test
     holds must find nothing (`crates/finder/src/targets.rs:416`), so (e) would need an
     existing test edited, which the constitution's Principle II forbids without the
     owner's approval.
   - **(f) each query word beginning a different word of the name**: `orion o` finds
     nothing, where (a) finds NGC 1976 and IC 434 (US1-S3). (a) is the simpler rule:
     (f) needs a matching of query words to name words.
2. **Marks and spaces** (FR-001; US1-S4, US1-S6, US1-S9, US1-S10). *Settled 2026-11-22:
   the owner's choice, "Rewrite to this".* Two things:
   - **In a common name, and in the query matched against it, only letters and digits
     are compared.** A word of a query or of a name is a piece between spaces, kept to
     its letters and digits. There is no list of marks: a hyphen, either apostrophe
     (`'` or the `’` that phone keyboards type), an `&` or any other is passed over
     alike, in the name and in the query. An id is looked up as today, marks and all
     (§ Edge Cases).
   - **A name also matches when the query reads along it**: the query's words, written
     with nothing between them, begin the name's words written the same way from one of
     the name's words on.

   So a name's marks and spaces need not be typed as the name has them:
   - `bow tie`, `bowtie` and `Bow-Tie` find NGC 40 (`Bow-Tie nebula`);
   - `cats eye`, `catseye`, `cat's eye` and `cat’s eye` find NGC 6543
     (`Cat's Eye Nebula`), and so does `cat eye nebula`, each word beginning a word;
   - `horse head nebula` finds B 33 (`Horsehead Nebula`), and `jewelbox` finds NGC 4755
     (`Herschel's Jewel Box`).

   Of the 182 common names, 5 hold a hyphen (`Bear-Paw`, `Bow-Tie`, `Eight-Burst`,
   `Fourcade-Figueroa`, `Wolf-Lundmark-Melotte`), 20 an apostrophe and 1 an `&`. What
   the rule changes for a query of one word is in § Clarifications and owner approval
   2. What it leaves as it is: a name's words begin where they begin today, after a
   space, so the part of a name after a hyphen finds nothing by itself, as today:
   `tie`, `paw`, `burst`, `lundmark`, `melotte` and `figueroa` (US1-S2 holds `tie`). A
   query with no letter or digit finds nothing (US1-S10). The alternatives, kept for
   the record:
   - **(a) marks stay inside words**, the first draft's recommendation: `bow tie` and
     `cats eye` find nothing.
   - **(b) names and queries split at hyphens as well as spaces, and lose `'` and
     `’`**, the draft's first reading of the owner's answer. It named two marks and
     treated each its own way, and `bowtie`, `catseye` and `horse head nebula` still
     found nothing. It changed what 11 of the 951 one-word prefixes find, `b`, `l`,
     `m`, `p` and `t` among them, through the words it made after hyphens (`Burst`,
     `Lundmark`, `Melotte`, `Paw`, `Tie`), and at the default limit of 20 `b` and `p`
     each lost an object to the one they gained (computed on 2026-11-21). The owner
     asked for a simpler rule.
   - **(c) reading along alone**, without each word beginning a word: `nebula orion`,
     `hercules cluster` and `bode galaxy` find nothing, against decision 1 (a).
3. **When the story may run** (front matter; plan.md § Refinement Record). *Settled
   2026-11-21: the recommended answer (a), and met on 2026-11-22.* (a): after 047 has
   landed: 047 stays in `depends_on_landed`, and the shared write of
   `crates/finder/tests/mcp/main.rs` is `merge_order`. 047 landed during the draft.
   Weighed against: (b) `concurrent` with 047's US3, so the story could run while it
   was building, which the validator takes as advisory only; or (c) drop US1-S8 and
   `crates/finder/tests/mcp/`, so the story writes only `targets.rs` and need not wait.
   No test would then send a query of several words through the route and the tool;
   016's `crates/finder/tests/mcp/session_tools.rs:59` holds the tool equal to the
   route for single words only.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **The search finds more than it did** (FR-001), in place of 012's FR-007 second
   bullet (`specs/012-targets-and-push-to/spec.md:640`): a query of several words that
   found nothing now finds the objects with a common name it matches. No canonical id
   and no Messier id written with a space finds anything else on this catalog (US1-S7).
   A designation with a space that is no id of the catalog can now find names: `C 30`
   finds NGC 2070 (`30 Dor Cluster`), `C 3` finds NGC 2070 and NGC 1049, and `C 4` and
   `C 47` find NGC 104 (§ Edge Cases). An observer who types one of those four Caldwell
   numbers that way gets a wrong object first where today they get nothing.
   `GET /api/targets` and `search_targets` keep their shapes.
2. **Six one-word queries that find something today change** (FR-001, owner decision
   2), in place of 012's FR-007 second bullet for those words. Five of today's 951
   one-word prefixes find one object more, or for `l'` ten (US1-S6): `le` gains
   NGC 6705; `sn` and `sno` gain NGC 6087; `l'` finds what `l` finds, and `l'e` what
   `le` finds. Under `le`, `sn`, `sno` and `l'` an object they gain comes first. `&`
   finds nothing, where it found C 14 (US1-S10). No one-word query that holds a letter
   or a digit stops matching an object it matched (FR-001). At a limit of 5 or more
   none of the 950 loses an object it showed; at 1, `le`, `sn`, `sno` and `l'` show a
   gained object in its place, and at 2 to 4 `l'` no longer shows NGC 6705 (computed,
   not tested). A one-word query that found nothing can now find an object where it
   passes over a mark or a space: `bowtie` and `catseye` (US1-S9), `cats` (computed).
3. **Ids keep their marks** (FR-001). This was added after the owner's choice of
   2026-11-22, whose words named no exception: only common names are matched by
   letters and digits. An id is looked up as today, so `ESO 56-115` needs its hyphen
   and `eso 56 115` finds nothing, as today (§ Edge Cases).

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
  under test: no test calls `search`, or a private function of the search, for an
  expected value.

### User Story 1 - A name of several words finds its object (Priority: P1)

As the observer, I type "Orion Nebula", or "Ring Nebula", or "Whirlpool Galaxy", in the
page's `Search catalog` field or through MCP, and the object comes back first, as it
does for "Orion" or "M42".

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
   `  Ring   Nebula `, `nebula orion`, `"nebula\torion"` (one query, written here as a
   Rust string: a tab, U+0009, between its two words), `gr or ne` and `whirl gal`,
   **Then** it asserts that the ids of these thirteen queries are, in turn, exactly
   `[NGC 1976]`, `[NGC 1976]`, `[NGC 1976]`, `[NGC 6720]`, `[NGC 5194]`,
   `[C 14]`, `[C 14]`, `[NGC 7000]`, `[NGC 6720]`, `[NGC 1976]`, `[NGC 1976]`,
   `[NGC 1976]` and `[NGC 5194]`.
2. **Given** the catalog, **When** a committed test searches with a limit of 20 for each
   of `orion galaxy`, `andromeda nebula`, `barbell cork`, `rion nebula`,
   `orion nebulae`, `tie`, `tie nebula`, `orion é` (with U+00E9) and `bow bow tie`,
   **Then** it asserts that each finds nothing.
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
   `[NGC 1, NGC 2, NGC 4, NGC 3]`.
5. **Given** every common name of every row of the extract that is neither `Dup` nor
   `NonEx` (the non-empty pieces of the row's `common_names` column, split at commas),
   **When** a committed test searches for each name in capitals with no limit
   (`usize::MAX`), **Then** it asserts that each search holds the object `find` gives
   for that row's `name` column, and that there are 182 names, counted once per row they
   appear in (175 of them distinct, which is not asserted).
6. **Given** every prefix of every word of those common names, lowercased, a word being
   what lies between spaces and commas as 012's rule reads them, **When** a committed
   test searches with no limit (`usize::MAX`) for each prefix that holds a letter or a
   digit, **Then** it asserts that there are 951 distinct prefixes, 950 of them holding
   a letter or a digit; that, for each of the 950, the set of its ids contains the
   canonical ids of the rows having a common name with a whitespace-separated word, as
   012's rule reads it, that starts with the query, compared in lowercase; and that the
   set holds besides those exactly these ids, and none for any other query: for `l'`,
   ESO 56-115, NGC 1980, IC 2944,
   NGC 6523, NGC 6618, NGC 6818, NGC 650, UGC 5470, NGC 6445 and NGC 6369; for `l'e`,
   UGC 5470; for `le`, NGC 6705; for `sn` and `sno`, NGC 6087.
7. **Given** every object of `targets::all()`, **When** a committed test searches with no
   limit (`usize::MAX`) for its canonical id and, for each object with a Messier id, for
   that id with a space after the `M` (the object's `messier` field, such as `M42`,
   written `M 42`), **Then** it asserts that each search's ids are exactly
   that object's id.
8. **Given** the console router, **When** a committed test asks the route and the tool for
   each of `Orion Nebula`, `Great Orion Nebula`, `Ring Nebula`, `Whirlpool Galaxy`,
   `Double Cluster`, `Cat’s Eye` (with U+2019, sent to the route as
   `Cat%E2%80%99s+Eye`) and `Horse Head`, **Then** it asserts that the route answers 200
   with a list whose ids are, in turn, exactly `[NGC 1976]`, `[NGC 1976]`, `[NGC 6720]`,
   `[NGC 5194]`, `[C 14]`, `[NGC 6543]` and `[B 33]`; that the tool's answer is not an
   error; that its `structuredContent` is `{"targets": <the route's answer>}`; and that
   it has one content block, whose text is JSON equal to the route's answer.
9. **Given** the catalog, **When** a committed test searches with a limit of 20 for each
   of `bow tie`, `bowtie`, `Bow-Tie`, `cats eye`, `catseye`, `catseye nebula`,
   `cat's eye`, `cat’s eye` (with U+2019), `cat eye nebula`, `bear paw`, `eight burst`,
   `Wolf Lundmark Melotte`, `Wolf-Lundmark-Melotte`, `stephans quintet`, `bode galaxy`,
   `horse head nebula`, `whirl pool`, `jewelbox` and `h chi persei`, **Then** it
   asserts that their ids are, in turn, exactly `[NGC 40]`, `[NGC 40]`, `[NGC 40]`,
   `[NGC 6543]`, `[NGC 6543]`, `[NGC 6543]`, `[NGC 6543]`, `[NGC 6543]`, `[NGC 6543]`,
   `[NGC 2537]`, `[NGC 3132]`, `[PGC 143]`, `[PGC 143]`, `[HCG 92]`, `[NGC 3031]`,
   `[B 33]`, `[NGC 5194]`, `[NGC 4755]` and `[C 14]`.
10. **Given** the catalog, **When** a committed test searches with a limit of 20 for each
    of `-`, `'`, `--`, `’` (U+2019), `' -` and `&`, **Then** it asserts that each finds
    nothing.

### Edge Cases

- **A query of one word** that holds a letter or a digit matches at least the objects
  it matched before: where the query begins a piece of a name, the query's letters and
  digits begin that piece's. It can match more only where a mark or a space is passed
  over, in the name or in the query, and the sort is untouched. US1-S6 checks the
  objects as sets; 012's tests of `crates/finder/src/targets.rs:384` and
  `crates/finder/tests/console/targets.rs:72`, unchanged, check the first result of the
  queries they name and the brightness order of `NEBULA`'s, and pass with the fix.
- **Spaces around and between words** do not matter: the query is split at whitespace,
  so leading, trailing and repeated spaces are passed over (US1-S1's
  `  Ring   Nebula `), and a tab separates words as a space does (US1-S1). An empty or
  blank `q` is still refused with 400 before the search
  (`crates/finder/src/api.rs:1658`).
- **Two query words may begin the same name word**: `orion o` finds NGC 1976 and then
  IC 434 (`Orion B`) (owner decision 1 (a); US1-S3).
- **A repeated query word is kept**: `bow bow tie` finds nothing, since `tie` begins no
  word of a name and `bowbowtie` reads along none (US1-S2).
- **Words from two names of one object** do not combine: `barbell cork` finds nothing,
  though M76 is both the Barbell Nebula and the Cork Nebula (US1-S2). Reading along
  stays within one name too.
- **Marks** need not be typed, and may be (owner decision 2; US1-S9): any character
  that is not a letter or a digit is passed over, in the name and in the query. `l'Ecu`
  is the word `lecu`, so `lecu` and `l'ecu` find NGC 6705, and `ecu`, as today, does
  not. A letter with an accent is a letter: in a query it is not passed over, so
  `orion é` finds nothing (US1-S2). That it is not folded to the letter without it, and
  that names and queries are lowercased alike, are read from the code, not tested: no
  name of the catalog has a letter outside ASCII. The rest is read from the rule, not
  tested, past US1-S9.
- **Ids keep their marks**, as today: only common names are matched by letters and
  digits, and the exact id is looked up as 012 does. `ESO 56-115` finds its object, and
  `eso 56 115` finds nothing, as today; 33 canonical ids hold a hyphen. Computed.
- **Reading along begins at the start of a word of the name**, any of them, and never
  inside one: `jewelbox` finds `Herschel's Jewel Box` from its second word (US1-S9),
  and `rion nebula` and `tie` find nothing (US1-S2). It runs over as many of the name's
  words as the query needs, and may end inside one: `catseye nebula` reads along three
  words of `Cat's Eye Nebula` (US1-S9), and `north american` finds NGC 7000, since
  `northamerican` begins `northamericanebula` (computed).
- **A query with no letter or digit** has no word and finds nothing (US1-S10); without
  that, every named object would match. The check comes before the exact id is looked
  up, which loses nothing: every id holds a letter or a digit.
- **Reading along can put another object first**: `sn`, typed toward `Snowball`, now
  finds NGC 6087 (`S Nor Cluster`) and then NGC 7662; `snow` finds NGC 7662 alone, as
  today. US1-S6 holds the objects; their order is computed, not tested.
- **Equal magnitudes** are ordered by canonical id, as today: `eagle nebula` gives
  IC 4703 before NGC 6611, which is M16 (US1-S3). Both carry the name in OpenNGC.
- **An id typed with a space that a name also matches** puts the exact id first and
  then the names by brightness (US1-S4): in its catalog `M 31` finds NGC 1, which is
  M31; then NGC 2 (magnitude 1), whose name holds both words; NGC 4 (3), whose name
  `Medium M31 Galaxy` it reads along from the second word; and NGC 3 (no magnitude).
  `M 3 Galaxy` does not match. No such id is in the catalog (US1-S7).
- **A designation typed with a space that is not a catalog id** can now find names
  holding its words: `C 30` finds NGC 2070 (`30 Dor Cluster`), `C 3` finds NGC 2070 and
  NGC 1049 (`Fornax Dwarf Cluster 3`), and `C 4` and `C 47` find NGC 104
  (`47 Tuc Cluster`), where all four find nothing today; no other Caldwell number from 1
  to 109 does. The catalog holds Caldwell ids only for objects without an NGC or IC id
  (`C 9`, `C 14`, `C 41`, `C 99`); Caldwell 30 and Caldwell 3 are NGC 7331 and NGC 4236,
  and NGC 104 is Caldwell 106 (by reading, not from the extract), so each of the four
  gives a wrong object first. None of the extract's 651 duplicate aliases, written with
  a space, finds a name. Read from the extract and the rule, not tested.
- **The page** sends what is typed as it is typed, trimmed, after each keystroke
  (`crates/finder/web/src/components/TargetSheet.tsx:57`). As a name is typed its name
  matches only narrow: over the 1,999 beginnings of the 175 distinct names, typed a
  character at a time, no added character adds a name match (computed). An exact id
  can still appear as words are added (`ngc` finds nothing, `ngc 1` finds NGC 1), as on
  `main`. An id being typed can now find names on the way. Of the beginnings of every
  canonical id and of every Messier id written with a space, three do: `cl 3`, typed
  toward `Cl 399`, finds NGC 2070 and NGC 1049 (`cl 39` finds nothing, and `cl 399` its
  object); `c 4`, toward `C 41`, finds NGC 104; and `hc`, toward `HCG 92`, finds C 14,
  reading along `h & chi Persei` (computed). Not run on the page.

## Requirements *(mandatory)*

### Functional Requirements

**The search** (US1):

- **FR-001**: `targets::search` SHALL answer, after any exact id, the objects with a
  common name that the query matches. The words of a query or of a name SHALL be the
  pieces between whitespace of its lowercase form (Rust's `str::to_lowercase`), each
  kept to its letters and digits (the characters for which `char::is_alphanumeric`
  holds), a piece left with none being no word (owner decision 2). A name SHALL match
  when each word of the query begins some word of the name, in any order (owner
  decision 1), or when the query's words, joined with nothing between them, begin the
  name's words joined the same way from one of the name's words to its last (owner
  decision 2). A query with no word
  SHALL find nothing. The exact id SHALL still be looked up from the trimmed,
  lowercased query as today. The order SHALL stay as it is: exact ids first, then by
  magnitude with no magnitude last (012's FR-007), then by canonical id
  (`crates/finder/src/targets.rs:194`); at most `limit` objects, with no duplicates. A
  query of one whitespace-separated piece that holds a letter or a digit SHALL match
  every object 012's rule matches for it, before the limit is applied.
- **FR-002**: The doc comment of `targets::search` SHALL state the rule of FR-001: exact
  ids first, then common names in brightness order; in a common name only letters and
  digits compared, whatever their case; a name matching when each word of the query
  begins one of its words, in any order, or when the query reads straight along the
  name from the start of one of its words.
- **FR-003**: Nothing else SHALL change. The story's changes are, in
  `crates/finder/src/targets.rs`, the empty-query check of `Catalog::search`
  (`crates/finder/src/targets.rs:166`), its filter, two private functions (one makes
  the words, one tells whether a name matches) and FR-002's doc comment; new tests
  after that module's last test,
  `every_source_object_is_reachable_by_its_raw_and_canonical_id`
  (`crates/finder/src/targets.rs:454`); the new module
  `crates/finder/tests/mcp/target_words.rs`; and its one `mod` line in
  `crates/finder/tests/mcp/main.rs`. `targets::find`, `targets::all`, the catalog's
  parsing and data, `crates/finder/src/api.rs`, `crates/finder/src/mcp/`, the page,
  `crates/finder/web/dist`, every other file under `crates/`, and every existing test
  and fixture are unchanged.

### Key Entities

- **A word**: a piece between whitespace of the lowercased query, or of one lowercased
  common name as OpenNGC gives it, kept to its letters and digits.
- **Reading along**: the query's words, joined, beginning the name's words, joined, from
  one of the name's words on.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited. Every existing
  test passes unchanged (plan.md § Shapes that existing tests pin).
- **SC-003**: Every common name of the catalog, typed in full, finds its object
  (US1-S5), where today every name of two or more words finds nothing.
- **SC-004**: Every one-word prefix of US1-S6 that holds a letter or a digit matches at
  least the objects it matched before, and more only as US1-S6 lists; every canonical
  id and every Messier id written with a space finds its object alone (US1-S7).
- **SC-005**: Each name of US1-S9 is found without its marks, and with its spaces left
  out or added, as US1-S9 lists; and with its marks (US1-S5 and US1-S9).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed.
  Every citation here was read at 28f436df.
- **The catalog is the pinned extract.** The counts 182, 951 and 950 and every expected
  list are read from `crates/finder/data/openngc.csv` at 28f436df; a new extract would
  need them read again.
- **Out of scope:**
  - letters with accents typed without them, and any folding beyond case;
  - the part of a name after a hyphen typed by itself (`tie`, `paw`);
  - ranking by how well a name matches, or Messier objects first on equal magnitude
    (`eagle nebula` keeps IC 4703 before M16);
  - misspellings and other forms of a word (`andromida galaxy`, `orion nebulae`), and
    star names, the Moon and planets, which 012 left out;
  - ids typed without their marks (`eso 56 115`): the id lookup is 012's, unchanged;
  - marking C5 done in `docs/voice/02-tool-contracts.md`, which is the owner's design
    note (plan.md § Outside the factory).
- **Outside the factory:** a look at the page's `Search catalog` field on
  `finder --simulate`, in plan.md § Outside the factory, by the owner. The console's
  binary changes, so a Pi kit built after 049 lands carries the fix.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's jobs run on the pull request and in the merge queue, outside the
  gates the judge sees, so no scenario's Then clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003]
```

One story: `Catalog::search`'s empty-query check and filter, two private functions,
`targets::search`'s doc comment, and their tests (plan.md § Plan traps 2).
