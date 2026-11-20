---
state: draft
depends_on_landed: ["014-goto", "015-tracking", "016-mcp", "017-console-stop-confirm", "030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "034-dark-and-flat-frames", "035-live-stack", "036-tracking-from-subs", "037-imaging-run", "038-track-follows-a-push", "040-stack-waits-for-steady", "041-imaging-run-on-the-rig"]
# The imaging run, third of three pieces: the run's block on the Imaging page, four MCP
# tools and the README's section with its field checklist (US9, US10, US11), over the run
# that 037-imaging-run and 041-imaging-run-on-the-rig build. Depends on 014 to 017, 030 to
# 038, 040 and 041 having landed. State: draft, re-anchored to `main` after 041 lands and
# before the flip. history.md has how it got here; flip.md what the flip needs.
---

# Feature Specification: The Imaging Run on the Page, over MCP and in the README

**Feature Branch**: `042-imaging-run-page-and-mcp`

**Created**: 2026-11-15, as part of `037-imaging-run`; a spec of its own since 2026-11-20

**Status**: Draft

**Input**: `037-imaging-run` and `041-imaging-run-on-the-rig` give the console one
imaging run, from a chosen target to a saved stack, served over three routes. This spec
puts it in front of the observer and of an MCP client: a block on the Imaging page that
starts a run, says what it is doing and stops it; four MCP tools with which a client
starts, watches, steers and stops a run, choosing the target and the settings but never
putting the steps in order itself; and a section of the README, with the checklist for
the first night at the telescope.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's scenarios run in
  `npm test`, a later command of the same gate, in `crates/finder/web`.
- **Paused time.** A test of a `Runner` or of the in-process console runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time. A test that
  starts the built `finder` is a plain `#[test]`.
- **The script**, **a scripted run**, **settling**, **the scripted console** and **R** are
  037's, and plan.md § What 037-imaging-run provides gives each as 037 defines it: the
  script's default answers and controls; a `Runner` over its parts on a `FixedClock` at
  Unix 1768528800; a settle of 1 s of virtual time; and 031's console rig with the
  script's parts added last, R being the child path `imaging` of the test's scratch
  folder.
- **Scratch folders.** A finder test module takes its folder from
  `crate::support::Scratch::new(name)` (`crates/finder/tests/support/mod.rs:12`), which
  makes the folder under the directory cargo names and removes it when dropped.
- **Requests** go through the router in-process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with `Content-Type:
  application/json` and no `Origin`. A scenario that names an `Origin` builds its
  request itself with a `Host` of localhost on port 8080. A foreign `Origin` is
  `http://elsewhere.example`.
- **Comparing a view.** A test compares `run` key by key, never whole, because later
  specs may add keys. `elapsed_s` is compared within 0.5 s, twice the run's poll.
- **Oracles.** Each scenario names its oracle: the simulator's truth; the session; the
  answer of another route of 030 to 036 at the same moment; or, for a scenario that
  checks only what the run says, calls and writes, the words, states, reasons and call
  lists the scenario itself states, which the test compares exactly. A test parses
  `run.json` and `stack.json` itself with `serde_json`, never through the console's
  code. A count or a word has no tolerance; every other tolerance is stated where the
  number is.


### User Story 9 - The run on the Imaging page (Priority: P2)

As the observer with a phone in the dark, I open the Imaging page, press one button, and
read one line that says what the run is doing, how many subs are stacked and dropped,
and who started it. At a focus pause the same block offers the two ways on.

**Why this priority**: At the telescope the page is the only thing to hand.

**Independent Test**: `npm test` in `crates/finder/web`, a command of the `test` gate,
runs `src/components/ImagingRunBlock.test.tsx`.

**Acceptance Scenarios**:

1. **Given** `ImagingRunBlock` rendered with a `ready` imaging view and no `run`,
   **Then** a committed test asserts one region named `Imaging run` holding a button
   `Start imaging run`, fields labelled `Subs to stack` and `Minutes to stack`, both
   empty, a checked checkbox `Keep every sub`, and no `status` or `alert`.
2. **Given** the block with a `run` whose `state` is `stacking`, `message`
   `Stacking M42: 12 stacked, 3 dropped.`, `subs`
   `{"stacked":12,"dropped":{"not_solved":2,"too_few_stars":1}}`, `elapsed_s` 125.4 and
   `by` `page`, **Then** a committed test asserts a status reading exactly
   `Stacking M42: 12 stacked, 3 dropped.`, the text
   `12 stacked · 3 dropped (not solved 2, too few stars 1)`, the text "Elapsed 2:05", a
   button `Stop imaging run` which, clicked once, reads `Tap again to stop the run` and
   calls no `fetch`, and clicked again calls `fetch` with `/api/imaging/run` and
   `{"method":"DELETE"}` and then `refresh`, no `Start imaging run` button, no button
   `Check focus again` or `Go anyway`, and no text `Started by an MCP client`. With
   `by` `mcp` the committed test asserts the text `Started by an MCP client`; with
   `elapsed_s` 125.6, that the text is still "Elapsed 2:05"; and with `subs`
   `{"stacked":5,"dropped":{}}`, that the text is exactly `5 stacked · 0 dropped`.
3. **Given** the block with a `run` whose `state` is `focus_pause` and whose `message`
   is that of 041's US6-S1, **Then** a committed test asserts an alert reading exactly that
   message, a button `Check focus again` that posts `{"anyway":false}` to
   `/api/imaging/run/continue`, a button `Go anyway` that posts `{"anyway":true}`, and
   the button `Stop imaging run`, which is not inside the element that holds the two
   pause buttons.
4. **Given** the block with a `run` whose `state` is `centring`, whose `message` is that
   of 041's US5-S1 and whose `warnings` hold the arcsecond warning of 037's US2-S3, **Then** a
   committed test asserts a status reading exactly the message and a paragraph reading exactly
   the warning.
5. **Given** the block with an ended `run` whose `ended` is
   `{"reason":"stopped","at":1768529000,"message":"Imaging run stopped. Track is still on.","saved":true,"track_on":true}`,
   with 12 stacked and the folder `runs/2026-01-16T02-00-10Z-M42`, **Then** a committed
   test asserts a status reading exactly
   `Imaging run stopped. Track is still on. 12 subs stacked · saved on the console in runs/2026-01-16T02-00-10Z-M42.`,
   a `Start imaging run` button and no alert; with 0 stacked, the status
   `Imaging run stopped. Track is still on. No sub was stacked.`; and with the reason
   `sun_up`, `track_on` false and the
   message `Ended: the Sun is up.`, an alert reading
   `Ended: the Sun is up. 12 subs stacked · saved on the console in runs/2026-01-16T02-00-10Z-M42.`
   and no status. With the reason `count_limit` and the message
   `Finished: the 12 subs asked for are stacked.`, the committed test asserts a
   status reading exactly
   `Finished: the 12 subs asked for are stacked. Saved on the console in runs/2026-01-16T02-00-10Z-M42.`
   and no alert; with the reason `duration_limit` and the message
   `Finished: the 120-minute limit was reached.`, a status reading exactly
   `Finished: the 120-minute limit was reached. 12 subs stacked · saved on the console in runs/2026-01-16T02-00-10Z-M42.`
   and no alert; and, for an ended run that has `warnings`, each warning as a
   paragraph.
6. **Given** 031's `ImagingPage` fed by `useImaging`, as 031's own page test feeds it,
   with `fetch` answering every read with a `ready` view holding the `run` of US9-S2,
   **Then** a committed test asserts that the region named `Imaging run` comes before
   the region named `Camera` in document order and shows the status of US9-S2; and that
   with a view whose `state` is `no_link` and which holds the ended `run` of US9-S5 the
   region still shows its status, while with `no_link` and no `run` the page has no
   region named `Imaging run`.
7. **Given** `DESIGN.md` and `.impeccable/design.json`, read by the test with
   `readWebFile` (`crates/finder/web/src/test/files.ts:11`) and the paths
   `../../../DESIGN.md` and `../../../.impeccable/design.json`, **Then** a committed
   test asserts that `DESIGN.md` holds a line that is exactly
   `### Imaging run — built`, and that the `components` array of the JSON holds exactly
   one entry whose `name` is `Imaging run`, with the `kind` `custom` and the `refersTo`
   `button` (oracle: the words of plan.md § `DESIGN.md` and `.impeccable/design.json`).
8. **Given** the block with the ended `run` of US9-S5, **Then** a committed test asserts
   that with `ended.saved` false, for the reason `stopped` as for `save_failed` with
   the message
   `Ended: the console could not save to its imaging folder: no space left on device. Check the disk.`,
   the block shows an alert, and no status, that ends exactly
   `12 subs were stacked but the stack was not saved: the subs are in runs/2026-01-16T02-00-10Z-M42/subs.`;
   that with 1 stacked and `saved` true the status ends
   `1 sub stacked · saved on the console in runs/2026-01-16T02-00-10Z-M42.`, and with
   `saved` false the
   alert ends
   `1 sub was stacked but the stack was not saved: the subs are in runs/2026-01-16T02-00-10Z-M42/subs.`;
   and that a paragraph reading exactly
   `Track is still on: the top bar's Stop stops the mount.` is shown while
   `ended.track_on` is true and not when it is false.
9. **Given** the block with the stacking `run` of US9-S2, under Vitest's fake timers,
   **Then** a committed test asserts that one click on `Stop imaging run` calls no
   `fetch` and leaves a button `Tap again to stop the run`; that 5 s later the button
   reads `Stop imaging run` again, and one click on it still calls no `fetch`; that a
   second click within the 5 s calls `fetch` once, with `{"method":"DELETE"}`; that
   while that `fetch` is unanswered the button reads `Stopping…` and is disabled; and
   that with a `run` whose `message` is `Saving the stack: 12 stacked.` the button
   `Stop imaging run` is disabled.
10. **Given** the block with the paused `run` of US9-S3, **Then** a committed test
    asserts that a 409 answer to `Check focus again` with
    `{"error":"the imaging run is not paused"}` shows a second alert with exactly that
    text and calls no `refresh`; that the block rendered again with a view whose
    `run.state` is `solving` no longer shows it; that while a `fetch` of
    `Check focus again` is unanswered that button reads `Checking…` and it, `Go anyway`
    and the stop button are disabled; and that when `fetch` rejects, the second alert
    reads exactly
    `The console did not answer. The run itself is not affected; try again.`
11. **Given** the block rendered with a `ready` imaging view whose `camera` is 1236 by
    822 pixels and whose `settings` hold `exposure_ms` 1000 and `binning` 1, and no
    `run`, **Then** a committed test asserts that `Minutes to stack` has the
    placeholder `120`; that the block holds the text
    `Goes to the target, checks focus, centres it on the imaging camera's sensor and stacks subs until you stop it or a limit is reached (two hours unless you give another).`
    and the text `About 14.6 GB of subs in 120 minutes at the current exposure.`; that
    with `30` typed into `Minutes to stack` the second reads
    `About 3.7 GB of subs in 30 minutes at the current exposure.`; and that with a view
    that also holds `unfinished_run` `runs/2026-01-15T03-00-00Z-M31` the block shows an
    alert reading exactly
    `The last imaging run did not finish: the console stopped while it ran. Its subs are in runs/2026-01-15T03-00-00Z-M31/subs.`
    and still the button `Start imaging run`.
12. **Given** the block with the paused `run` of US9-S3 whose `started_at` is
    1768528800, `elapsed_s` 25.4 and `pause_ends_at` 1768529700, **Then** a committed
    test asserts the text "Focus aid open below · 14:34 left"; and, with the stacking
    `run` of US9-S2 given `started_at` 1768528800, `stacking_since` 1768528860 and
    `settings.duration_s` 7200, the text "Stacking for 1:05 of 120:00".
13. **Given** the block of US9-S1, **When** a test clicks `Start imaging run`, with
    `fetch` answering 200 and a view, **Then** a committed test asserts that `fetch` was
    called with `/api/imaging/run` and
    `{"method":"POST","headers":{"Content-Type":"application/json"},"body":"{\"keep_subs\":true}"}`,
    then `refresh`; that with `50` and `30` typed and the checkbox cleared the body is
    `{"subs":50,"duration_s":1800,"keep_subs":false}`; that with `0` typed in both
    fields the body is `{"keep_subs":true}`; that `50` typed into `Subs to stack`, the
    block then rendered again with a new view object, as `useImaging`'s one-second read
    gives it, and the button clicked, posts `{"subs":50,"keep_subs":true}`; and that a
    409 answer with
    `{"error":"no target is selected"}` shows an alert with exactly that text and calls
    no `refresh`, and that the block rendered again with a view holding the `run` of
    US9-S2 no longer shows that alert.

---

### User Story 10 - Four MCP tools (Priority: P2)

As an MCP client, I set the target, start an imaging run, read it until it stacks, and
stop it. I pass the same guards as the page, and I cannot ask for any one step out of
turn.

**Why this priority**: "Ask for the Orion Nebula and get a picture" is asked by voice or
chat as often as by a tap.

**Independent Test**: `crates/finder/tests/mcp/imaging_run.rs`, the module `imaging_run`
of finder's `mcp` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test mcp imaging_run::`.

**Acceptance Scenarios**:

1. **Given** the scripted console served through the MCP door, as
   `crates/finder/tests/support/mcp.rs:28` serves a state, **When** a committed test
   reads `tools/list`, **Then** it asserts that the list holds `start_imaging_run`,
   `stop_imaging_run`, `continue_imaging_run` and `get_imaging_run`, each with the
   description plan.md § The MCP tools gives and an `outputSchema` of
   `{"type":"object"}`; that only `get_imaging_run` carries `readOnlyHint: true`; and
   that the input schema of `start_imaging_run` names `subs`, `duration_s` and
   `keep_subs`, and that of `continue_imaging_run` names `anyway`.
2. **Given** the same console with a call log the test reads, **When** a committed test
   calls `get_imaging_run`, then `start_imaging_run` with `{"subs":5}`, then
   `get_imaging_run`, **Then** it asserts that the first answers `{"run":null}`; that
   the start answers a `run` with `by` `mcp` and `settings.subs` 5; that the last
   answers the same `run`, key for key but for `elapsed_s`, `state`, `message`, `focus`,
   `centring` and `folder`; that the script's `calls()` hold `mount.point mcp`; and
   that the log holds exactly one line, beginning
   `mcp start_imaging_run {"subs":5} -> `.
3. **Given** the same console, **Then** a committed test asserts tool errors with these
   words: `start_imaging_run` with `{"subs":0}`,
   `subs must be a whole number from 1 to 100000`; `stop_imaging_run` with `{"x":1}`,
   `stop_imaging_run takes no arguments`; `continue_imaging_run` with `{"anyway":true}`
   and no run, `no imaging run is running`; and on a console without
   `with_imaging_run_steps`, `start_imaging_run` with `{}`, `stop_imaging_run` with
   `{}` and `continue_imaging_run` with `{"anyway":true}` each answer
   `this console runs no imaging runs` while
   `get_imaging_run` answers `{"run":null}`. `start_imaging_run` with
   `{"exposure_ms":1}` and `continue_imaging_run` with `{}` each answer a result with
   `isError` true, and `continue_imaging_run` with `{"anyway":true,"x":1}` answers a
   tool error beginning ``unknown field `x` ``. The test leaves 1 s of virtual time
   between two calls of
   `start_imaging_run`, so the rate limit of US10-S4 is not what answers.
4. **Given** the same console with `refuse(Part::Solver, "this console has no imaging solver")`,
   **When** a committed test calls `start_imaging_run` three times within one second of
   virtual time, **Then** it asserts that the first two answer the tool error
   `this console has no imaging solver`, the words `POST /api/imaging/run` answers with
   409 on the same console, and the third
   `the rate limit of 2 moving calls a second was reached`; and that the log holds three
   lines.
5. **Given** the same console with a run started over MCP that is stacking, after
   `set_stack(4, &[], false)` and a settle, **When** a committed test calls
   `stop_imaging_run`, **Then** it asserts a `run` whose `subs.stacked` is 4, a
   `run` with `state` `ended` and `ended.reason` `stopped`, and no new line in the call
   log.
6. **Given** `crate::support::mcp::sim::Rig` (`crates/finder/tests/support/mcp.rs:182`)
   after a sync, its state given `with_imaging(R)` and
   `with_imaging_run_steps(script.steps())` and served again through the door, **When**
   a committed test calls `move_axis` for one axis at 0.5°/s for 30 s and then
   `start_imaging_run`, **Then** it asserts the tool error
   `a direct axis move is running` and an empty `calls()`; that after `stop` and a
   wait of 3 s `start_imaging_run` is accepted; and that `stop_imaging_run` then
   answers a `run` ended `stopped` while the rig's recording mount receives no
   `StopAll` in the next second (oracle: the recording mount).
7. **Given** the scripted console, **When** a committed test calls each of the four
   tools once with valid arguments, **Then** it asserts that no answer, and no line of
   the call log, contains `test-token`.
8. **Given** the same console with a run paused as in 041's US6-S1, **When** a committed test
   calls `continue_imaging_run` with `{"anyway":true}` and settles, **Then** it asserts
   that `get_imaging_run` answers a `run` whose state is no longer `focus_pause` and
   whose `warnings` are `["Focus was not checked again: the observer went on anyway."]`
   (the tool itself answers the view as the resume is handed over, still `focus_pause`).

---

### User Story 11 - The README and the field checklist (Priority: P3)

As the owner on the first night with the whole chain, I follow a written checklist from
"camera on the focuser" to "a saved stack", started once from the page and once by an
MCP client, and I know what each state and each ending means.

**Why this priority**: The run cannot be proven at a desk; the checklist is how the
field checks are carried out and written down.

**Independent Test**: `crates/finder/tests/console/imaging_run_docs.rs`, the module
`imaging_run_docs` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_docs::`.

**Acceptance Scenarios**:

1. **Given** `crates/finder/README.md` read by the test from
   `CARGO_MANIFEST_DIR`, **Then** a committed test asserts that it holds the heading
   `## An imaging run`, and that the text from that heading to the next `## ` heading,
   or to the end of the file,
   holds, each between backticks, every word of `finder::imaging::run::STATES` and every
   word of `finder::imaging::run::ENDINGS`, the three routes, the four tool names, and
   `run.json`; and that the text from that heading to
   `### Field checklist: the first imaging run` contains `corner`, `7200` and
   `What to do next`, the heading of the fourth column of its table of endings.
2. **Given** the same section, **Then** a committed test asserts that it holds the
   heading `### Field checklist: the first imaging run`, and that the text from that
   heading to the next heading, or to the end of the file, holds exactly eighteen lines
   that begin with a number and a full stop, numbered 1 to 18 in order, and names
   `Start imaging run`, `start_imaging_run`, `continue_imaging_run`, `Stop` and
   `run.json`; and that the text from the line that
   begins `8.` to the line that begins `9.` contains the word `corner` (the known limit
   of the run's own imaging frames, FR-040).
3. **Given** the same file, **Then** a committed test asserts that its `## API` table
   holds a row for each of `POST /api/imaging/run`, `DELETE /api/imaging/run` and
   `POST /api/imaging/run/continue`, and that the text of `## MCP clients` names the
   four tools.

---


### Edge Cases

- **Every edge case of a run** is 037's or 041's (their spec.md § Edge Cases). The page
  and the tools show and pass on what those describe, and add none of their own.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-035**: The Imaging page SHALL show a block headed Imaging run, first on the page,
  whenever the imaging view is `ready` or holds a `run`. With no run on it SHALL offer
  `Start imaging run` with the two optional limits, the second showing its default of
  120 minutes, `Keep every sub`, and an estimate of the size of the subs at the
  current exposure; and, while the view holds `unfinished_run`, it SHALL say that the
  last imaging run did not finish and where its subs are. While one is on
  it SHALL show the run's message as a status (an alert in `focus_pause`), the counts,
  the elapsed time, the time stacked against the limit once it stacks,
  `Started by an MCP client` when `by` is `mcp`, each warning, and
  `Stop imaging run`, which SHALL ask for a second tap within 5 s before it stops the
  run; in `focus_pause` also the time the pause has left, and `Check focus again` and
  `Go anyway` in a row of their own. While a request of the block is unanswered its
  buttons SHALL be disabled; a refusal SHALL show its words, and a request that got no
  answer SHALL say so in the block's own words. After
  an ending it SHALL say why it ended and what was saved: `saved on the console in` only
  when
  `ended.saved` is true, and otherwise that the stack was not saved and where the subs
  are. It SHALL be a status for `stopped`, `count_limit` and `duration_limit` and an
  alert for every other reason and whenever subs were stacked and the stack was not
  saved; and while `ended.track_on` is true it SHALL say that Track is still on and
  that the top bar's Stop stops the mount (owner approval 6).
- **FR-036**: `crates/finder/web/src/api.ts` SHALL give the run's type and the three
  calls; `DESIGN.md` SHALL describe the block under the heading
  `### Imaging run — built`; `.impeccable/design.json` SHALL list one component named
  `Imaging run`; and `crates/finder/web/dist` SHALL be the build of its source.
- **FR-037**: The console SHALL offer the MCP tools `start_imaging_run`,
  `stop_imaging_run`, `continue_imaging_run` and `get_imaging_run`, each passing through
  to the action its route calls, refusing with the route's words, and answering
  `{"run": …}` with the run's view or null (owner approval 3).
- **FR-038**: `start_imaging_run` SHALL be a moving tool: it SHALL take the rate limit
  first, start the run by `mcp` through `start_motion` with the tool's own name, be
  refused with `a direct axis move is running` while one runs, and write one call-log
  line. The other three SHALL take no rate limit and write no line (owner decision 19).
- **FR-039**: The `## API` table of `crates/finder/README.md` SHALL hold a row for each
  of the three routes, and its `## MCP clients` SHALL list the four tools (owner
  approval 7).
- **FR-040**: `crates/finder/README.md` SHALL gain `## An imaging run`, saying what a
  run does in order, each state and each ending by its word, with what the mount is
  doing after each ending and what to do next, the limits, the three
  routes and four tools, where `run.json` lies and what it holds, that the imaging
  camera is held throughout, by the run, the focus aid or the stack, that the run moves
  the mount only through Track and stops it only when it is itself stopped while still
  slewing, what Stop does, what a console that exits mid-run leaves behind, that the
  imaging exposure should be at least as long as the finder camera's, and that the run's
  own imaging frames are taken at the camera's exposure setting while only the stack's
  subs are capped (owner approval 7).
- **FR-041**: That section SHALL hold `### Field checklist: the first imaging run`,
  eighteen numbered steps from the camera on the focuser to a saved stack, with Track
  left alone on the target first, runs started from the page and by an MCP client, a
  Stop while slewing and while stacking, and what to write down at each,
  whether the corner stars of the run's solved imaging frames are trailed among them.

### Key Entities

- **State**: where a run is, one of ten words.
- **Ending**: why a run is over, one of seventeen reasons, with a message for the
  observer.
- **The run's record**: `run.json`, in the stack's folder or in `runs/<run id>/` under
  the imaging folder.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or `playwright.config.ts` is edited, 037's and 041's test files
  among the existing tests. The only edits to existing files under `crates/finder/tests`
  are one `mod` line for each new test module, in alphabetical order, in its binary's
  `main.rs`: `imaging_run_docs` in `crates/finder/tests/console/main.rs` and
  `imaging_run` in `crates/finder/tests/mcp/main.rs`. No other line of those files and no
  other existing test changes.
- **SC-003**: No `Cargo.toml`, `Cargo.lock` or `deny.toml` changes, no scope-link message,
  config key, console flag or glossary term is added, and no `unsafe` is written.
- **SC-007**: Every story's diff, measured as plan.md § Plan traps 3 says, is under
  64 KiB.


## Assumptions

- **Earlier features are in place.** Every spec this one names in `depends_on_landed` has
  landed before it is built, 037 and 041 among them (flip.md § Prerequisites 1). Every
  `path:line` here was read at 49bb210. A file that 030 to 041 create is named without a
  line, with the spec that creates it.
- **Out of scope:**
  - a `target` argument (037's owner decision 10);
  - listing, fetching or deleting runs through the API, and returning the stacked
    picture from a run tool: the picture is 035's `get_stack`;
  - MCP on the scope node.
- **Outside the factory:** the field checks at the telescope, which follow the checklist
  of FR-041, and a design review of the run block on a phone in the dark (flip.md
  § Field checks).
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.


## Work Graph

```yaml
US9:
  depends_on: []
  implements: [FR-035, FR-036]
US10:
  depends_on: []
  depends_on_merged: [US9]
  implements: [FR-037, FR-038]
US11:
  depends_on: []
  depends_on_merged: [US10]
  implements: [FR-039, FR-040, FR-041]
```

Every edge is a merge edge, and the stories run one after another, after the whole of
`041-imaging-run-on-the-rig` has landed. An edge names each earlier story of this spec
that last wrote a file the story writes next, so that the later story builds on the
merged file; a file 037 or 041 last wrote is on `main` when this spec starts.

- **US9** calls 037's three routes and reads every state and ending 037 and 041 make. It
  names no story.
- **US10** calls the same actions as the routes. It writes `crates/finder/src/api.rs`
  next after 041's US8, and follows US9 so that one story is on the floor at a time.
- **US11** documents all of it and tests the document against 037's two word lists; it
  writes `crates/finder/tests/console/main.rs` next after 041's US7, and follows US10
  because its README names the four tools and three routes as they were built.
