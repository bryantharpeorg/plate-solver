---
description: "Task list for 042-imaging-run-page-and-mcp: The Imaging Run on the Page, over MCP and in the README"
---

# Tasks: The Imaging Run on the Page, over MCP and in the README

**Prerequisites** (flip.md § Prerequisites): every spec in spec.md's `depends_on_landed`
has landed and this trio is re-anchored to `main`; no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
  The one exception is the `mod` line of each new finder test module, added in
  alphabetical order to the binary's `main.rs` your story's tasks name, which holds no
  test (spec.md SC-002). Change nothing else there. A test file 037, 041 or
  an earlier story of this spec created is an existing test: add your tests in the new file your
  tasks name, and leave every earlier scenario passing (plan.md § Plan traps 24).
- A new finder test is a module file in the binary's folder plan.md § Technical Context
  names, never a file at the top level of `crates/finder/tests` or in
  `crates/finder/tests/support`, either of which fails the `layout` test. It takes its
  scratch folder from `crate::support::Scratch`, and holds only the setup its own tests
  use, since an unused helper fails `clippy -D warnings`. A rig test builds its own rig
  in its own module.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Tests type in the words, reasons, states and call lists spec.md gives; none builds an
  expected message by calling `Ending::message` (037's US12's test calls it, and compares what
  it answers with a sentence typed in), and none reads `run.json` or
  `stack.json` through the console's code (plan.md § Plan traps 6).
- Use the stand-in parts of `finder::imaging::run::script` for every part your story
  does not make real. Add no method to a part's trait and no field to `Steps`, do not
  change `Runner::new`, `start`, `stop`, `resume` or `view`, and after 037's US12 do not write
  `script.rs` (plan.md § Shapes that existing tests pin).
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 3 says.
- Every code citation is `path:line` as read at 49bb210; a file 030 to 038, 040 or 041
  create is named without a line. If a cited line has moved, re-read it rather than guess.
- Add no dependency and change no `Cargo.toml`. Write no `unsafe`. Change nothing under
  `crates/simulator`, `crates/scope-node` or `crates/scope-link`.
- Do not edit `specs/037-imaging-run/`, `specs/041-imaging-run-on-the-rig/` or
  `specs/042-imaging-run-page-and-mcp/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 9: User Story 9 — The run on the Imaging page (Priority: P2)

### Tests first

- [ ] T036 [US9] Create `crates/finder/web/src/components/ImagingRunBlock.test.tsx` with
  failing tests for US9-S1, US9-S2, US9-S3, US9-S4, US9-S5 and US9-S6, stubbing `fetch`
  as 031's Imaging page tests do, with the cases each scenario lists: a limit typed as
  `0`, a draft that survives a new view, no pause buttons outside a pause, the elapsed
  time rounded down, no brackets with none dropped, a limit's ending as a status and an
  ended run's warnings; and for US9-S7, reading `DESIGN.md` and
  `.impeccable/design.json` with `readWebFile` from `src/test/files.ts`. Add failing
  tests for the ended line's forms when the stack was not saved, `1 sub`, and the line
  that Track is still on (US9-S8), the second tap of `Stop imaging run` under fake
  timers, the button while its request is out and while the run saves its stack
  (US9-S9), a refused or unanswered `Check focus again` and when its alert goes
  (US9-S10), the start form's placeholder, quiet line, size estimate and the line of
  an unfinished run (US9-S11), and the time a pause has left and the time stacked
  against the limit (US9-S12), and what a click on `Start imaging run` posts, with
  a refusal's alert gone once a run appears (US9-S13).

### Implementation

- [ ] T037 [US9] Add `ImagingRun`, `ImagingRunRequest`, `postImagingRun`,
  `deleteImagingRun` and `postImagingRunContinue` to `crates/finder/web/src/api.ts`,
  and `run` to `ImagingView`. Create
  `crates/finder/web/src/components/ImagingRunBlock.tsx` with the words of plan.md
  § The page, keeping its two fields and its checkbox in its own state, and render it
  in `crates/finder/web/src/components/ImagingPage.tsx` right after the title block.
  The three calls throw the block's own words when `fetch` rejects; the type gains
  `stacking_since`, `limit_at`, `pause_ends_at`, `ended.saved` and `ended.track_on`,
  and the view
  `unfinished_run`.
- [ ] T038 [US9] Add the Imaging run entry to `DESIGN.md` and the Imaging run component
  to `.impeccable/design.json` (plan.md § `DESIGN.md` and `.impeccable/design.json`).
  Run the `lint` and `test` gates with every existing Vitest and Playwright test
  unchanged and green, commit the rebuilt `crates/finder/web/dist`, and commit the
  green story (US9-S1, US9-S2, US9-S3, US9-S4, US9-S5, US9-S6, US9-S7, US9-S8, US9-S9,
  US9-S10, US9-S11, US9-S12 and US9-S13).

## Phase 10: User Story 10 — Four MCP tools (Priority: P2)

### Tests first

- [ ] T039 [US10] Create `crates/finder/tests/mcp/imaging_run.rs`, the module
  `imaging_run` of finder's `mcp` test binary, with `mod imaging_run;` in its
  alphabetical place in `crates/finder/tests/mcp/main.rs`, serving the scripted console
  through the door as `crates/finder/tests/support/mcp.rs` serves a state, with a call
  log the test reads. Write failing tests for the list (US10-S1), a start and a read
  (US10-S2), the errors, each tool of a console without parts called with arguments it
  would otherwise accept, and an unknown key of `continue_imaging_run` (US10-S3), the
  rate limit (US10-S4), stop (US10-S5) and continue, read back with
  `get_imaging_run` after a settle (US10-S8), a start during a direct axis move on
  `crate::support::mcp::sim::Rig` (`crates/finder/tests/support/mcp.rs:182`), and
  `stop_imaging_run` sending the recording mount no `StopAll`
  (US10-S6), and the token (US10-S7). Leave 1 s of virtual time between two starts but
  in US10-S4.

### Implementation

- [ ] T040 [US10] In `crates/finder/src/mcp/tools.rs`, add `ImagingRunArguments`,
  `ImagingRunContinueArguments` and the four tools with the descriptions, order of
  checks, parsing and answers of plan.md § The MCP tools, `start_imaging_run` answering
  and logging `runner.view()` as read once `start_motion` has answered. In
  `crates/finder/src/api.rs`, add `start_imaging_run` to the one `matches!` of
  `start_motion` plan.md § `start_motion`'s one line names. The README is US11's.
- [ ] T041 [US10] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US10-S1, US10-S2, US10-S3, US10-S4, US10-S5, US10-S6,
  US10-S7 and US10-S8).

## Phase 11: User Story 11 — The README and the field checklist (Priority: P3)

### Tests first

- [ ] T042 [US11] Create `crates/finder/tests/console/imaging_run_docs.rs`, the module
  `imaging_run_docs` of `console`, with its `mod` line in
  `crates/finder/tests/console/main.rs`, reading `README.md` from `CARGO_MANIFEST_DIR`.
  Write failing tests for the section's words, with `corner`, `7200` and
  `What to do next` in its prose before the checklist (US11-S1), the checklist's
  eighteen steps
  and the word `corner` in its step 8 (US11-S2), each text read to the next heading or
  to the end of the file, and the three rows of the API table
  and the four tools in MCP clients (US11-S3).

### Implementation

- [ ] T043 [US11] In `crates/finder/README.md`, add the three rows to the `## API`
  table, the four tools to `## MCP clients` with the count in the sentence before its
  list made four higher, and the sections `## An imaging run` and
  `### Field checklist: the first imaging run`, with the rows, the content and the
  eighteen steps of plan.md § The README: rows, tools, the section and the field
  checklist, every state word and ending reason between backticks, and the known limit
  that the run's own imaging frames are not capped as subs are, in the prose and in
  step 8. The prose also says, as that section lists them: the four sentences of
  `stopped` and what the mount does after each, what to do next after each ending,
  in a fourth column of the table of endings headed `What to do next`, the waits and
  counts as they now are,
  that the imaging exposure should be at least as long as the finder camera's (and
  step 4 of the checklist says so), what a console that exits mid-run leaves, and the
  reserve file.
- [ ] T044 [US11] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US11-S1, US11-S2 and US11-S3).

## Execution Order

```text
US9 ──► US10 ──► US11
```

- **`037-imaging-run`** and **`041-imaging-run-on-the-rig`** have landed before US9.
- **US9** touches only `crates/finder/web/`, `DESIGN.md` and `.impeccable/design.json`,
  and reads every state and ending the stories of 037 and 041 make.
- **US10** writes `crates/finder/src/api.rs` after 041's US8.
- **US11** comes last: it writes the README for the routes and tools as they were
  built, and its test reads it.
- **`[P]`** marks nothing here: the stories form one chain.
