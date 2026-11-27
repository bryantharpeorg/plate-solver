---
description: "Task list for 056-console-opens-to-a-paired-phone: The Console Opens to a Paired Phone"
---

# Tasks: The Console Opens to a Paired Phone

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at f6d4f5fd;
- 052, 053, 054 and 055 have landed, in that order, and this trio is re-anchored on that
  `main`, and on 057 or 058 if either lands first: not met;
- the owner decisions in spec.md are settled: not met, none of the ten is answered (the
  flip ask);
- every story has been probe-built from this text: met, probe round 1 on `probe/056-r1`
  (tip 6fe4f173: 0 blocking, 16 clarity), folded and proven on that branch at e4335286;
- the page half has run at each story's tip: met in probe round 1 and at the fold's tip
  e4335286 (`CI=1 npm run e2e`), to be run again on the re-anchored base;
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `Cargo.toml`, `Cargo.lock`,
  `crates/finder/web/playwright.config.ts` or `.specify/`. If an existing test fails,
  stop and escalate with its exact output. A test file an earlier story of this spec
  created is an existing test, the rig and the unit tests in
  `crates/finder/src/pairing.rs` among them (plan.md § Plan traps 2). The exceptions:
  each story's `mod` lines in `crates/finder/tests/console/main.rs`,
  `crates/finder/tests/mcp/main.rs` or `crates/finder/tests/cli/main.rs`, each in its
  alphabetical place, and US1's rig at the end of `crates/finder/tests/support/mod.rs`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder/web/src/`, `crates/finder/web/dist`, `crates/finder/src/session.rs`,
  `crates/finder/src/link.rs`, `crates/finder/src/mcp/mod.rs`,
  `crates/finder/src/mcp/tools.rs`, `crates/scope-link`, `crates/scope-node`, `proto/`
  and the plate solver's crates are never touched, and no file is added under
  `crates/finder/tests/support/` (027's layout test pins its files).
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the prompt's
  page, the READMEs', the manuals' and the glossary's among them. Say "finder console"
  or "console", never "finder" alone, in every message and label.
- Never print, log or return the console token in a body. Tests use
  `test-console-token` only.
- No test downloads anything, binds anything but loopback, or writes outside
  `CARGO_TARGET_TMPDIR`. Run no tool the gates don't run, and do not set
  `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are probe round 1's, from
  this text alone (plan.md § What was run and what was read), where it ran them.

## Phase 1: User Story 1 - Another device presents the console token before any route answers (Priority: P1)

### Tests first

- [ ] T001 [US1] At the end of `crates/finder/tests/support/mod.rs`, add the rig
  `pub mod pairing { ... }` as plan.md § Interfaces gives it. Create
  `crates/finder/tests/console/pairing.rs` with one test for each of US1-S1 to US1-S7
  under `#[tokio::test]`, through `use crate::support::pairing::*;`, building the gated
  console, the plain console, the phone and the reads as spec.md § User Scenarios
  defines them, and add `mod pairing;` to `crates/finder/tests/console/main.rs` in its
  alphabetical place. Before T002 and T003 none of finder's four test binaries that
  include `support` (`console`, `mcp`, `motion` and `cli`) builds, since the rig in the
  shared `support/mod.rs` names `finder::pairing` and `with_console_token` (run).

### Implementation

- [ ] T002 [US1] Create `crates/finder/src/pairing.rs` with its module doc, `COOKIE`,
  `WRONG_TOKENS`, `WRONG_WINDOW_S`, `MAX_PEERS`, `ConsoleToken` (`new`, `matches`),
  `normalize` with Crockford's look-alikes, `InvalidToken`, `Pairing` and its `check`
  (times later than now dropped), `AppState::with_console_token`, `loopback`, `peer`,
  `presented` (the scheme in any case), the three words' constants and `gate`
  (`/api/stop`, and `/mcp` only while MCP is on, passed first; the Host check before the
  loopback pass), and the unit test of `ConsoleToken::new` and `matches`, each as plan.md
  § Interfaces gives it (FR-001 to FR-006).
- [ ] T003 [US1] In `crates/finder/src/api.rs`, add the `pairing` field after
  `allowed_hosts` and `pairing: None` in `AppState::shared`, make the `mcp` field
  `pub(crate)`, and split `router` into the gated `router` and `console_routes`; in
  `crates/finder/src/lib.rs`, add `pub mod pairing;` after `pub mod imaging;` and the doc
  paragraph, as plan.md § Interfaces gives them (FR-001 to FR-006).
- [ ] T004 [US1] Run the `lint` and `test` gates, `CI=1 npm run e2e` among them, with
  every existing test unchanged and green, then commit the green story (FR-001 to
  FR-006; US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6, US1-S7).

## Phase 2: User Story 2 - The phone pairs once, through the console's prompt (Priority: P1)

### Tests first

- [ ] T005 [US2] Create `crates/finder/tests/console/pairing_prompt.rs` with one test for
  each of US2-S1 to US2-S5, US2-S5 under `#[tokio::test(start_paused = true)]` on the
  live console and the others under `#[tokio::test]`, using the rig through `use
  crate::support::pairing::*;` and `crate::support::live` and writing the set and cleared
  cookies and DESIGN.md's five night colours in the test, and add `mod pairing_prompt;`
  to `crates/finder/tests/console/main.rs` in its alphabetical place. Before T006 the
  five tests fail (run).

### Implementation

- [ ] T006 [US2] In `crates/finder/src/pairing.rs`, add `COOKIE_MAX_AGE_S`, `FORM_LIMIT`,
  the gate's `/pair` pass, prompt and cleared cookie, `routes(&state)` (empty without a
  token), `prompt`, `pair`, `form_field`, `prompt_html` with plan.md § Interfaces' page
  exactly, and a unit test of `form_field`; in `crates/finder/src/api.rs`, start
  `console_routes` from `crate::pairing::routes(&state)`; in `crates/finder/src/lib.rs`,
  add the `/pair` route line and `/pair` to the paragraph (FR-007 to FR-010).
- [ ] T007 [US2] Run the `lint` and `test` gates, `CI=1 npm run e2e` among them, with
  every existing test unchanged and green, then commit the green story (FR-007 to
  FR-010; US2-S1, US2-S2, US2-S3, US2-S4, US2-S5).

## Phase 3: User Story 3 - MCP from another device, and the binary's console token (Priority: P1)

### Tests first

- [ ] T008 [US3] Create `crates/finder/tests/mcp/pairing.rs` with US3-S1 and US3-S2
  under `#[tokio::test]` on the MCP rig, importing the rig's constants by name next to
  `crate::support::mcp::*`, and add `mod pairing;` to `crates/finder/tests/mcp/main.rs`
  in its alphabetical place. Create `crates/finder/tests/cli/pairing.rs` with US3-S3 and
  US3-S4 under `#[test]`, starting the built binary on 127.0.0.1:0 as spec.md § User
  Scenarios defines it, waiting at most 60 s for each start and for the bad-token start's
  exit (plan.md § Plan traps 6), and add `mod pairing;` to
  `crates/finder/tests/cli/main.rs` in its alphabetical place. Create
  `crates/finder/web/e2e/pairing.spec.ts` with US3-S6 as plan.md § Interfaces gives it.
  Before T009 and T010 the four Rust tests fail (run) and the browser test finds no
  prompt at `/pair` (run).

### Implementation

- [ ] T009 [US3] In `crates/finder/src/pairing.rs`, add `TOKEN_FILE`, `ALPHABET`,
  `generate`, `grouped`, `load_or_create` with its warning for a shared file,
  `write_owner_only` through a temporary file, `listening_hint`, `admits_mcp` and the
  module doc's paragraph on the binary, with new unit tests of `grouped`, `generate` and
  `listening_hint` (US3-S5); in `crates/finder/src/mcp/door.rs`, add the three lines
  after the peer check (FR-011, FR-012).
- [ ] T010 [US3] In `crates/finder/src/main.rs`, load the token after the session,
  printing a bad file's error with `eprintln!` and exiting 1, pass
  `with_console_token` after `with_allowed_hosts`, and print `listening_hint`'s line in
  place of the loopback hint's `if`, as plan.md § Interfaces gives them (FR-012).
- [ ] T011 [US3] Run the `lint` and `test` gates, `CI=1 npm run e2e` among them, with
  every existing test unchanged and green, then commit the green story (FR-011, FR-012;
  US3-S1, US3-S2, US3-S3, US3-S4, US3-S5, US3-S6).

## Phase 4: User Story 4 - The READMEs, the manuals and the glossary say how a phone opens the console (Priority: P2)

### Tests first

- [ ] T012 [US4] Create `crates/finder/tests/console/pairing_docs.rs` with US4-S1, US4-S2
  and US4-S3 under `#[test]`, reading each page with every run of white space as one
  space, the sentences, the absent sentences and the entries written in the test, and
  add `mod pairing_docs;` to `crates/finder/tests/console/main.rs` in its alphabetical
  place. Before T013 the three fail (run).

### Implementation

- [ ] T013 [US4] In `crates/finder/README.md`, change the `--listen` and `--allow-host`
  rows, add `### The console token` after `### From a phone`'s location paragraph and
  change the MCP sentence; in `README.md`, `docs/safety-and-security.md` and
  `docs/guide/console.md`, replace the passages plan.md § Interfaces names; in
  `GLOSSARY.md`, change **MCP client** and add **Console token** after it; each exactly
  as plan.md § Interfaces gives it (FR-013, FR-014).
- [ ] T014 [US4] Run the `lint` and `test` gates, `CI=1 npm run e2e` among them, with
  every existing test unchanged and green, then commit the green story (FR-013, FR-014;
  US4-S1, US4-S2, US4-S3).

## Execution Order

```text
US1 -> US2 -> US3 -> US4
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`, re-anchored
  on 052, 053, 054 and 055, and on 057 or 058 if either lands first.
- **US2** starts once US1 has merged: it adds the prompt to US1's `pairing` module and
  its tests use US1's rig.
- **US3** starts once US2 has merged: it adds the token file and MCP's check to the same
  module, and its browser test needs the binary's token, which it makes.
- **US4** starts once US3 has merged: its words describe what US3 makes, and its `mod`
  line follows US2's.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
