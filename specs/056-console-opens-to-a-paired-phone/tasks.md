---
description: "Task list for 056-console-opens-to-a-paired-phone: The Console Opens to a Paired Phone"
---

# Tasks: The Console Opens to a Paired Phone

**Prerequisites** (plan.md § Prerequisites, each with its evidence there):
- every spec in `depends_on_landed` has landed on `main`: met at f6d4f5fd;
- 052, 053, 054 and 055 have landed, in that order, and this trio is re-anchored on that
  `main`: not met;
- the owner decisions in spec.md are settled: not met, none of the nine is answered (the
  flip ask);
- every story has been probe-built from this text: not met; only the drafting probe on
  `probe/056` has run;
- the page half has run once: met on the drafting probe (`CI=1 npm run e2e`), to be run
  again on the re-anchored base;
- no operator-staged file is needed: met.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `Cargo.toml`, `Cargo.lock`,
  `crates/finder/web/playwright.config.ts` or `.specify/`. If an existing test fails,
  stop and escalate with its exact output. A test file an earlier story of this spec
  created is an existing test, the unit tests in `crates/finder/src/pairing.rs` among
  them; US1's test helpers are `pub(super)` from the start so US2 uses them without
  editing it (plan.md § Plan traps 2). The exceptions: each story's `mod` lines in
  `crates/finder/tests/console/main.rs`, `crates/finder/tests/mcp/main.rs` or
  `crates/finder/tests/cli/main.rs`, each in its alphabetical place.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/finder/web/src/`, `crates/finder/web/dist`, `crates/finder/src/session.rs`,
  `crates/finder/src/link.rs`, `crates/finder/src/mcp/mod.rs`,
  `crates/finder/src/mcp/tools.rs`, `crates/scope-link`, `crates/scope-node`, `proto/`
  and the plate solver's crates are never touched, and nothing is added under
  `crates/finder/tests/support/`.
- Copy the words of spec.md's FRs and of plan.md § Interfaces exactly, the prompt's
  page, the README's and the glossary's among them. Say "finder console" or "console",
  never "finder" alone, in every message and label.
- Never print, log or answer the console token. Tests use `test-console-token` only.
- No test downloads anything, and no test writes outside `CARGO_TARGET_TMPDIR`. Run no
  tool the gates don't run, and do not set `CARGO_TARGET_DIR`.
- The results said here before each story's implementation are the drafting probe's
  (plan.md § What was run and what was read), where it ran them.

## Phase 1: User Story 1 - Another device presents the console token before any route answers (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/pairing.rs` with one test for each
  of US1-S1 to US1-S5 under `#[tokio::test]`, building the gated console, the plain
  console, the phone and the reads as spec.md § User Scenarios defines them, with the
  words, the cookie and the bearer written in the test and its helpers `pub(super)`
  (plan.md § Plan traps 2), and add `mod pairing;` to
  `crates/finder/tests/console/main.rs` after `mod notices;`. Before T002 and T003 the
  binary does not build (no `finder::pairing`; run).

### Implementation

- [ ] T002 [US1] Create `crates/finder/src/pairing.rs` with its module doc, `COOKIE`,
  `WRONG_TOKENS`, `WRONG_WINDOW_S`, `MAX_PEERS`, `ConsoleToken` (`new`, `matches`),
  `InvalidToken`, `Pairing` and its `check`, `AppState::with_console_token`, `loopback`,
  `peer`, `presented`, the three words' constants and `gate`, and the unit test of
  `ConsoleToken::new` and `matches`, each as plan.md § Interfaces gives it (FR-001 to
  FR-006).
- [ ] T003 [US1] In `crates/finder/src/api.rs`, add the `pairing` field after
  `allowed_hosts` and `pairing: None` in `AppState::shared`, and split `router` into the
  gated `router` and `console_routes`; in `crates/finder/src/lib.rs`, add `pub mod
  pairing;` after `pub mod imaging;` and the doc paragraph, as plan.md § Interfaces gives
  them (FR-001 to FR-006).
- [ ] T004 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-006; US1-S1, US1-S2, US1-S3, US1-S4,
  US1-S5).

## Phase 2: User Story 2 - The phone pairs once, through the console's prompt (Priority: P1)

### Tests first

- [ ] T005 [US2] Create `crates/finder/tests/console/pairing_prompt.rs` with one test for
  each of US2-S1 to US2-S4 under `#[tokio::test]`, using US1's helpers through `use
  super::pairing::*;` and writing the set and cleared cookies and DESIGN.md's five night
  colours in the test, and add `mod pairing_prompt;` to
  `crates/finder/tests/console/main.rs` after `mod pairing;`. Create
  `crates/finder/web/e2e/pairing.spec.ts` with US2-S5 as plan.md § Interfaces gives it.
  Before T006 the four Rust tests fail (run).

### Implementation

- [ ] T006 [US2] In `crates/finder/src/pairing.rs`, add `COOKIE_MAX_AGE_S`, `FORM_LIMIT`,
  the gate's `/pair` pass, prompt and cleared cookie, `routes`, `prompt`, `pair`,
  `form_field`, `prompt_html` with plan.md § Interfaces' page exactly, and a unit test
  of `form_field`; in `crates/finder/src/api.rs`, start `console_routes` from
  `crate::pairing::routes()`; in `crates/finder/src/lib.rs`, add the `/pair` route line
  and `/pair` to the paragraph (FR-007 to FR-010).
- [ ] T007 [US2] Run the `lint` and `test` gates, `CI=1 npm run e2e` among them, with
  every existing test unchanged and green, then commit the green story (FR-007 to
  FR-010; US2-S1, US2-S2, US2-S3, US2-S4, US2-S5).

## Phase 3: User Story 3 - MCP from another device, and the binary's console token (Priority: P1)

### Tests first

- [ ] T008 [US3] Create `crates/finder/tests/mcp/pairing.rs` with US3-S1 and US3-S2
  under `#[tokio::test]` on the MCP rig, and add `mod pairing;` to
  `crates/finder/tests/mcp/main.rs` after `mod motion;`. Create
  `crates/finder/tests/cli/pairing.rs` with US3-S3 and US3-S4 under `#[test]`, starting
  the built binary as spec.md § User Scenarios defines it, waiting at most 60 s for each
  start and for the bad-token start's exit (plan.md § Plan traps 6), and add `mod
  pairing;` to `crates/finder/tests/cli/main.rs` after `mod mount_sky_night;`. Before
  T009 and T010 all four fail, the bad-token start by its own 60 s wait (run: before the
  wait, by nextest's 300 s limit).

### Implementation

- [ ] T009 [US3] In `crates/finder/src/pairing.rs`, add `TOKEN_FILE`, `ALPHABET`,
  `generate`, `grouped`, `load_or_create`, its owner-only write, `admits_mcp` and the
  module doc's paragraph on the binary, with new unit tests of `grouped` and `generate`;
  in `crates/finder/src/mcp/door.rs`, add the three lines after the peer check (FR-011,
  FR-012).
- [ ] T010 [US3] In `crates/finder/src/main.rs`, load the token after the session, pass
  `with_console_token` after `with_allowed_hosts`, and print the off-loopback line, as
  plan.md § Interfaces gives them (FR-012).
- [ ] T011 [US3] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-011, FR-012; US3-S1, US3-S2, US3-S3, US3-S4).

## Phase 4: User Story 4 - The README and the glossary say how a phone opens the console (Priority: P2)

### Tests first

- [ ] T012 [US4] Create `crates/finder/tests/console/pairing_docs.rs` with US4-S1 and
  US4-S2 under `#[test]`, reading the README with every run of white space as one space,
  the sentences and entries written in the test, and add `mod pairing_docs;` to
  `crates/finder/tests/console/main.rs` after `mod pairing;`. Before T013 both fail (run).

### Implementation

- [ ] T013 [US4] In `crates/finder/README.md`, change the `--listen` row, add
  `### The console token` after `### From a phone`'s location paragraph and change the
  MCP sentence; in `GLOSSARY.md`, change **MCP client** and add **Console token** after
  it; each exactly as plan.md § Interfaces gives it (FR-013, FR-014).
- [ ] T014 [US4] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-013, FR-014; US4-S1, US4-S2).

## Execution Order

```text
US1 -> US2 -> US3 -> US4
```

- **US1** starts from a `main` that holds every spec of `depends_on_landed`, re-anchored
  on 052, 053, 054 and 055.
- **US2** starts once US1 has merged: it adds the prompt to US1's `pairing` module and
  its tests use US1's helpers.
- **US3** starts once US2 has merged: it adds the token file and MCP's check to the same
  module.
- **US4** starts once US3 has merged: its words describe what US3 makes, and its `mod`
  line follows US2's.
- **`[P]`** marks nothing here: each story's implementation follows its tests.
