# Implementation Plan: An Imaging Abort Answers in Time

**Branch**: `043-imaging-abort-answers-in-time` | **Date**: 2026-11-20 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

One call in the console's imaging code gets a time limit. `Imaging::abort` waits for the
scope node's answer to `AbortExposure` for at most `ABORT_WAIT`, 5 s. A call the node
has not answered by then is dropped and ends the frame `aborted`, as a failed call
already does. Every stop that awaits the abort, 035's stack, 033's focus aid and 034's
calibration set, and through them the routes and MCP tools, then answers on a node that
has stopped answering, with no change of its own. The README's rows say so. No node,
protocol, route shape or view shape changes.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI.

**Primary Dependencies**: nothing new. `tokio::time::timeout`, which `mod.rs` already
uses for `CALL_WAIT` and `SET_WAIT`.

**Storage**: none.

**Testing**:
- `crates/finder/tests/console/imaging_abort_silent.rs` (new), a module of finder's
  `console` binary, under `#[tokio::test(start_paused = true)]`.
- Its rigs are the `pub(crate)` items of two existing modules of the same binary, read
  and not edited: `calibration_console` (`console_rig`, `sim_rig`, `request`,
  `wait_until`, `Rig`) and `stack_loop` (`stand_in_rig`, `ScriptedSolver`), as
  `calibration_darks.rs` and `stack_camera.rs` already use them.
- No Vitest or Playwright test: the page is unchanged.

**Target Platform**: the `finder` console. The scope node is not touched.

**Constraints**:
- `ImagingView`, `FrameView`, `StackView` and every route's answer keep their shapes.
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test moves real hardware.

**Scale/Scope**: one story, one source file, the README, one new test file and one
`mod` line.

## Constitution Check

*GATE: checked at the start of the story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, star detection or `crates/math-core`. The behaviour is grounded in 031's FR-013, 033's FR-028, 035's FR-018 and FR-032, and in what the probe of 2026-11-20 ran; what rests on reading is listed in § What was run and what was read, and none is invented beyond it. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. The new tests go in a new file; the one `mod` line is spec owner approval 1. No tolerance of an existing test moves. § Shapes that existing tests pin names every existing test file beside the change. |
| III. Tests first, independent expectations | The oracles are the routes' answers, `Imaging::ended`, the stack's file on disk, the scenarios' words and numbers, and paused Tokio time supplied to the node and the console. 5,000 ms is the scenarios' number, never `ABORT_WAIT` read from the code. The tests fail on `main` (§ What was run and what was read). |
| IV. Critical paths | No `unsafe`, no dependency, `Cargo.toml` or `Cargo.lock` change, nothing under `crates/scope-link/proto/` or `crates/grpc-service`, no `build.rs`. |
| V. Proven open source first, honest licensing | Nothing is designed that existing software could supply: the change is a time limit on one call, with tokio's own `timeout`. No reused code, no new license. |

**Shared helpers.** The constitution says a finder test reaches shared helpers through
`crate::support`. This module also uses `pub(crate)` rigs of two sibling modules, as two
existing modules do; it adds nothing to `support/` and edits neither sibling. A story
that finds this refused escalates.

**Mount neutrality.** Nothing here reads a mount, an axis or a mount type. The mount's
Stop keeps its own limits (`crates/finder/src/link.rs:384`).

## Project Structure

```text
crates/finder/
├── README.md                                   # US1: the abort's and the stack's rows, the abort paragraph
├── src/imaging/mod.rs                          # US1: ABORT_WAIT, the bound in Imaging::abort
└── tests/console/
    ├── main.rs                                 # US1: one mod line (owner approval 1)
    └── imaging_abort_silent.rs                 # US1, new
```

## Interfaces

### `crates/finder/src/imaging/mod.rs` (US1)

A constant after `SET_WAIT` (`crates/finder/src/imaging/mod.rs:49`):

```rust
/// The limit on the node's answer to the abort of an exposure (043).
pub const ABORT_WAIT: Duration = Duration::from_secs(5);
```

The `Exposing` branch of `Imaging::abort` (`crates/finder/src/imaging/mod.rs:798`),
with no other line of the function changed:

```rust
Stop::Exposing(mut client, task) => {
    // The task ends the frame when the node's `Aborted` arrives;
    // a failed call, or one not answered within `ABORT_WAIT`, ends
    // it here (043).
    if !matches!(time::timeout(ABORT_WAIT, client.abort()).await, Ok(Ok(()))) {
        task.abort();
        self.end_aborted();
    }
}
```

Dropping the timed-out future drops the call (owner decision 2). The state's lock is not
held across the await: `stop` is taken under it and the lock released before the
`match`, as on `main`. The function's doc comment gains, as its last sentence: "A call
the node fails, or does not answer within `ABORT_WAIT`, ends the frame here too, and the
call is dropped (043)."

### `crates/finder/README.md` (US1)

Three places, nothing else:
- the row of `POST /api/imaging/abort` (`crates/finder/README.md:233`), before "Same-origin
  writes only.": "A node that does not answer the abort within 5 s is given up on: the
  frame ends `aborted` and the call answers then.";
- the row of `DELETE /api/imaging/stack` (`crates/finder/README.md:243`), before "Same-origin
  writes only.": "On a node that has stopped answering, the abort of the sub in flight is
  given up after 5 s and the stop answers then.";
- the paragraph on the abort, after "no file written for it." (`crates/finder/README.md:668`),
  wrapped at 80 columns like the paragraph, with the busy words' code span on one line:
  "A node that does not answer `AbortExposure` within 5 s, as over a link that has gone
  quiet, is given up on: the console ends the frame `aborted` itself and drops the call.
  The node's exposure may then run on to its end, and a frame asked for before that fails
  with `the exposure failed: an exposure is already in progress`."

## What was run and what was read

**Run on 2026-11-20, before the draft**, on the branch `probe/043` cut from `main` at
fc161a54, in paused time, three times each with the same outcome:

- the nine scenarios' tests on `main`'s code: seven fail and two pass. The abort, the
  first of two aborts, the stack's stop, the focus aid's stop and the dark set's stop had
  not answered after 60 s; the abort answered 8 s late answered after 8 s; the stack's
  stop behind a waiting abort answered after 28.9 s. US1-S2 and US1-S5, a node that
  answers within the bound, pass on `main`, as they must;
- the trial fix as § Interfaces gives it, with the README text: all nine pass;
- with the fix, the `lint` gate's four commands and the `test` gate's two cargo
  commands: every test of the workspace passed, the console binary's 388 existing tests
  among them, unchanged;
- what the console shows while the link is down and once it is back, and what became of
  the node's exposure (spec Clarifications), from the tests and from one exploratory run
  that kept the node silent for 12 s after the abort's answer, with the same outcome;
- the Edge Case of the node answering again in the instant the console gave up, once;
- the nine tests at the probe's tip with both gates' Rust commands (§ Verification).

**Run in probe round 1, on 2026-11-20**, from this trio's text alone, on a branch cut
from the trio's own commit: the claims above about `main` and the fix, again, with the
same outcome; and these mutations of the fix, each caught by a scenario:
- the bound removed, or at 10 s, 4.9 s or 5.2 s: US1-S1, US1-S3, US1-S4 and US1-S7, and
  at every value but 5.2 s US1-S6, US1-S8 and US1-S9 too;
- a bound that always waits its full length: US1-S2 and US1-S5;
- the frame not ended on a timeout: US1-S1, US1-S4 and US1-S6 to US1-S9;
- the timed-out call left running in a spawned task, and the frame ended on a timeout
  with its task left running: US1-S4 as it now reads, whose words are those of the
  round's case that passed three times on the code and failed under both; the first also
  US1-S1.

Two mutations no scenario catches rest on the diff (§ Read from the diff): an answered
call that also ends the frame at the console, and the `Receiving` branch waiting too.
US1-S3's second abort is sent 3 s after the silence, since the console reads `no_link`
within 3 s and the round saw it at 910 ms.

**Read, not run**:
- the node's own copy of an abandoned exposure with `save_at_node`, and its frame count
  (spec Clarifications);
- a node that answers the abort and never sends `Aborted` (spec Edge Cases);
- an abort reaching the node late over a real network, and what option (b) of owner
  decision 2 would do;
- the focus aid's settings put back to a node that is silent but still reads `ready`
  (spec Edge Cases);
- the MCP tools `set_stacking` and `set_focus_aid` with `on` false, which call the same
  `Stacker::stop` and `Imaging::focus_stop` as the routes;
- a stack started after an abort given up on, ending `camera` (owner decision 1);
- every bound listed in § Other calls on the stop paths.

**Not run at all**: the real link and node, the page, and the page half of the `test`
gate, `npm ci` and what follows it in `crates/finder/web`, which installs packages; the
story writes nothing the page is built from.

## The rig and a quiet link

`set_unresponsive(true)` makes every call wait at the node's controls
(`crates/scope-node/src/service.rs:78`): `AbortExposure`
(`crates/scope-node/src/imaging/service.rs:355`), heartbeats and the camera's own calls.
`Expose` waits there too, but only before it starts
(`crates/scope-node/src/imaging/service.rs:278`): an exposure already under way runs on,
and its header and chunks still reach the console over the imaging connection. So on the
rig the frame of an abort that hangs is still saved at the end of its exposure, which is
why 035's view read `ended` while its stop hung, as 041's probe saw. A real quiet link
also silences the stream; there, on `main`, the frame waits out its header's limit. The
scenarios assert the abort's answer and the frame's state before the exposure would end,
so they hold for both.

## Other calls on the stop paths

Every call to the node that a stop, an abort or a stack's end awaits in 031, 033, 034,
035 and 036's code at fc161a54, and in what 040 added by 6d79292b, for the seat. Only the first is unbounded, and only it
is in scope.

| Call | Awaited by | Bound |
|---|---|---|
| `AbortExposure` (`crates/finder/src/imaging/mod.rs:801`) | every stop (spec Clarifications) | none; FR-001 |
| `Expose` and the header (`crates/finder/src/imaging/frames.rs:303`) | `Imaging::ended`, which the stack's taker (`crates/finder/src/imaging/stack/service.rs:849`), the aid (`crates/finder/src/imaging/focus/run.rs:903`, `crates/finder/src/imaging/focus/run.rs:1055`) and the set (`crates/finder/src/imaging/calibration/service.rs:741`) await | twice the exposure plus 150 s (`crates/finder/src/imaging/frames.rs:32`); a stop's abort ends the frame first |
| each chunk (`crates/finder/src/imaging/frames.rs:389`) | the same | 30 s, `CHUNK_WAIT`; a frame `receiving` is ended at once by an abort |
| `ConfigureImaging` through `Lease::set` (`crates/finder/src/imaging/mod.rs:663`) | the aid's settings put back (`crates/finder/src/imaging/focus/run.rs:1058`) | 30 s, `SET_WAIT`; refused at once once the camera reads `no_link` |
| `ConfigureImaging` through `Lease::set` | the stack's sub length, set before its first sub and put back at its end (040; `crates/finder/src/imaging/stack/service.rs:818`, `crates/finder/src/imaging/stack/service.rs:903`) | 30 s, `SET_WAIT`; refused at once once the camera reads `no_link`, which on a silent node it does before `ABORT_WAIT` runs out (US1-S6 passes at 5,000 ms with it) |
| the imaging task's own calls (`crates/finder/src/imaging/mod.rs:985`) | none of the stops | 5 s, `CALL_WAIT` |
| StopAll (`crates/finder/src/link.rs:384`) | the mount's Stop | 200 ms a try within 2 s |

Not node calls: the stack's worker solves with the console's own solver, and its files
are written on the blocking pool (`crates/finder/src/imaging/stack/service.rs:567`). The
calibration set puts back only the console's own exposure (`crates/finder/src/imaging/calibration/service.rs:404`).
036's `Steering::before_exposure` (`crates/finder/src/imaging/steering.rs:368`), the one
await 036 adds before an exposure, is awaited by the stack's taker since 040
(`crates/finder/src/imaging/stack/service.rs:808`), raced, `biased`, against the stack's
stop: it is no call to the node, and a stop ends it.

## What this gives later specs

Once 043 has landed, 041 and 042 may rely on these, and need neither probe's stand-in:

- `Imaging::abort` answers within `ABORT_WAIT` whether or not the node answers, and the
  frame has then ended `aborted`. 041's camera part's abort, which awaits it, answers on
  a lost link.
- `Stacker::stop` answers within `ABORT_WAIT` and the stack's save on a node that has
  stopped answering, with the stack `ended` and `stopped`. 041's stack part can call
  `stacker.stop().await` as its trio first wrote it, without the race against the
  stack's own `ended` view that 041-R17's stand-in added, and 041's run's ending
  finishes on a lost link.
- So 042's `stop_imaging_run` and `DELETE /api/imaging/run`, which wait on the run's
  ending, answer on a lost link, as 042-R3 asked. The point 041-R17 and 042-R3 proposed
  for each flip.md's Prerequisites 2 becomes "043 has landed".

043 does not edit 041's or 042's trio; the seat folds this into them.

## Read from the diff, not from a test

- **The README's three places.** No test reads them. Copy them from § Interfaces.
- **The doc comment of `Imaging::abort`.** Copy it from § Interfaces.
- **That the call is dropped, not left running** (owner decision 2). US1-S4's frame
  id+1, asked 4 s after the abort's answer, after the camera's late abort would have
  landed, fails busy: the call was dropped, not left running. Its view 30 s later still
  shows frame id+1 and no `frames` folder: the frame's task ended with the call. That the
  future is dropped rather than detached also shows in the diff: no `spawn` around
  `client.abort()`.
- **That an answered call leaves the frame to its task** (FR-002). It shows in the
  diff: `task.abort()` and `end_aborted` run only when the answer is not `Ok(Ok(()))`.
  No scenario has a node that answers the abort and then sends no `Aborted` (spec
  § Assumptions, out of scope).
- **That a frame `receiving` is stopped at once, as today** (FR-002). It shows in the
  diff: T002 changes no line of the `Receiving` branch.

## Shapes that existing tests pin

Every existing test passes unchanged with the story built (run on the probe). The files
beside the change, and what the story does to what they observe:

- **`crates/finder/tests/console/imaging_control.rs`.** 031's US5-S2
  (`crates/finder/tests/console/imaging_control.rs:389`) aborts a 30 s exposure on a
  node that answers and sees `aborted` within 1 s: the call is answered in time, so
  nothing changes.
- **`crates/finder/tests/console/stack_loop.rs`.** The stop that aborts a long exposure
  within 1 s (`crates/finder/tests/console/stack_loop.rs:585`): answered in time. Its
  `stand_in_rig` and `ScriptedSolver` are used, not edited.
- **`crates/finder/tests/console/calibration_console.rs`.** Its rig and helpers are used,
  not edited.
- **`crates/finder/tests/console/stack_console.rs`, `stack_camera.rs`,
  `calibration_darks.rs` and `imaging_focus_aid.rs`.** Their stops and aborts run on a
  node that answers: `stopping_during_frames_or_before_the_offset_releases_the_lease`
  (`crates/finder/tests/console/calibration_darks.rs:740`) still stops within 2 s, and
  the aid's lost-link ending in `imaging_focus_aid.rs` asks for no abort.
- **`crates/finder/tests/console/imaging.rs`.** 031's dropped link
  (`crates/finder/tests/console/imaging.rs:742`) takes no frame.
- **`crates/finder/tests/mcp/imaging.rs`** and the other MCP and CLI tests: no shape
  changes, and no abort there goes unanswered.
- **`crates/finder/tests/layout/main.rs`.** The new module sits in `console/` with one
  `mod` line, and adds no helper file.
- **`crates/finder/tests/console/main.rs`.** One line is added (owner approval 1).

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Bound the one call in `Imaging::abort` | Bound each stop (the stack's, the aid's, the set's) | One call is the cause; a bound per stop leaves the route's own abort hanging and repeats itself four times. |
| Bound the call | 041's stand-in: race the stack's stop against its own `ended` view | The stand-in fixes only 041's run; 035's, 033's and 034's routes and the frame's abort still hang (041 owner point, "Instead"). |
| End the frame as a failed call does | A new frame state, such as `abandoned` | It would change `FrameView`'s words, which the page, MCP and every taker read; the node's exposure is just as unknown after a failed call. |
| 5 s | 2 s or 10 s | Owner decision 1. |
| Drop the call | Leave it running in a task of its own | Owner decision 2: a call that lands after the link comes back stops whatever exposure is then in progress. |
| A constant of its own | Reuse `CALL_WAIT` | `CALL_WAIT` is the imaging task's limit and has its own words; the abort's bound is an owner decision of its own. |
| Answer the view as today | An error, or a field saying the node did not confirm | FR-004: no route's or view's shape changes; the frame ends `aborted` either way. |
| One story | A story for the bound and one for the stops | The stops change no code; their tests are the bound's evidence. Measured at 39.1 % of 65,536 B together (§ Plan traps 2). |

## User Story 1 - The abort and the stops above it

Tests first: `imaging_abort_silent.rs` and its `mod` line; on `main` seven of its nine
tests fail (§ What was run and what was read). Then `ABORT_WAIT` and the bound in
`Imaging::abort`, and the README's three places.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 6d79292b, unchanged since fc161a54):
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

1. **The earlier specs have landed:** met. Every spec in `depends_on_landed` had landed at
   6d79292b, 040 among them (2026-11-20).
2. **040 has landed, and `main` is still as read.** Met at 6d79292b, on 2026-11-20: 040
   is in `depends_on_landed`; the record's `shared_writes` name 040's US11 for the README
   and 040's US8 for `crates/finder/tests/console/main.rs`, the last to write each, and
   036's US5 still for `crates/finder/src/imaging/mod.rs`; every citation was re-read at
   6d79292b, fifteen moved and none changed its meaning; § Other calls on the stop paths
   gains the two calls 040 added; and the probe's tests and trial fix, put onto
   6d79292b unchanged, pass with every finder test. What the draft said before 040
   landed: every citation here was read at
   fc161a54. 040's stories write `crates/finder/README.md` (its US7, US8 and US11),
   `crates/finder/tests/console/main.rs` (US6, US8),
   `crates/finder/src/imaging/stack/service.rs` and `crates/finder/src/mcp/tools.rs`.
   Its trio is not on `main` at fc161a54, and the validator refuses a
   `depends_on_landed` edge to a spec it cannot find (`dangling_dep`), so 040 is not
   named there. Once 040 has landed: add `040-stack-waits-for-steady` to
   `depends_on_landed`; add to the record's `shared_writes` the README (040's US11) and
   `crates/finder/tests/console/main.rs` (040's US8); re-read every citation in the
   files above and in `crates/finder/src/imaging/mod.rs`; re-run the validator and the
   probe's tests on the new `main`. If the owner wants 043 before 040 instead, 040's
   stories merge their rows and `mod` lines over 043's.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Met for the decisions: both settled on 2026-11-20 with
   the recommended answers ("Yes, take them"); the approvals are given by the flip.
4. **Probe builds.** Met for round 1. The drafting probe of 2026-11-20 built the
   story's code and tests from the draft's design. Probe round 1, the same day, built
   US1 from this trio's text alone: seven of nine failed on `main` as T001 says and all
   nine passed after, on five runs; both gates' Rust commands passed with every existing
   test unchanged; the diff measured 26,789 B (40.9 % of 65,536 B, 104.5 % of the
   estimate). It found no blocking defect, and its seven clarity defects are folded
   here.
5. **No operator-staged files are needed.**

## Outside the factory

A field check on the real link, by the owner, written here and not run. With the node
rebuilt as it is, the console rebuilt with 043, the imaging camera connected and the page
open:

1. Take a 60 s imaging frame. 10 s in, take the Pi off its network (switch its Wi-Fi off
   or pull its cable). Within 3 s the page shows the camera not connected.
2. Press Abort. It should answer within about 5 s.
3. Put the Pi back on its network 30 s into the exposure. Once the page reads the camera
   ready, the frame reads `aborted`. Take a 1 s frame at once: note whether it fails with
   `an exposure is already in progress` (the node's exposure ran on) or is saved (the
   abort reached the node late). Take another once the 60 s have passed: it is saved.
4. Start a stack with 60 s subs and, mid-sub, take the Pi off its network and press
   Stop. The Stop should answer within about 5 s with the stack `stopped`. Put the Pi
   back, start a stack again, and note whether its first sub ends `aborted` with no
   abort asked for: that would be a late abort from the old connection (spec Edge
   Cases).

A Stop that does not answer within 10 s at step 2 or 4 means the bound is not in
force: restart the console once the link is back, and report it.

## Plan traps

They bind the story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* the story's diff stays under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. The probe's commits
   measured 25,639 B, 39.1 % of 65,536 B: the test file 16,346 B, the README 6,998 B
   (its rows are single long lines, so each changed row counts twice), `mod.rs` 1,901 B
   and the `mod` line 394 B. Probe round 1, built from this trio, measured 26,789 B,
   40.9 %: the test file 17,496 B and the rest to the byte. At 1.35 times the drafting
   probe's, the factor 041's second probe report applies to a factory build, it would be
   about 34,600 B, 52.8 %; the folded US1-S4 adds a few lines to the test file.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/043-imaging-abort-answers-in-time/` or `.specify/`.
4. **The view outside `ready` holds no frame.** On a silent node the abort's answer is
   `{"state":"no_link"}`. *Check:* the frame's end on a silent node is read through
   `Imaging::ended`, or once the node answers again; never from the answer.
5. **The cancel must reach the node before it answers again** (spec Edge Cases).
   *Check:* US1-S1 keeps the node silent 1 s after the abort's answer, and reads the
   view then.
6. **Paused time and the blocking pool.** A stop that writes the stack's files may show
   time passing during that work. *Check:* the stops' upper bounds are 500 ms above
   5,000 ms; the abort's own are 100 ms.
7. **An oracle copied from the code proves nothing.** *Check:* the tests write 5,000,
   3,000 and the busy words as literals; none reads `ABORT_WAIT`.
8. **A negative assertion passes when nothing happens.** *Check:* US1-S1's failed frame
   id+1 stands beside the saved frame id+2, and US1-S4's beside US1-S5's saved frame.
9. **The answered path is unchanged.** *Check:* US1-S2 asserts the answer within
   100 ms; a bound that always waits fails it.
10. **An unused helper fails clippy.** *Check:* the test file holds only the helpers its
    own tests use.
11. **A new finder test joins `console`.** *Check:* the module is
    `crates/finder/tests/console/imaging_abort_silent.rs` with one `mod` line after
    `mod imaging;`.
12. **rustdoc denies warnings.** *Check:* `ABORT_WAIT` is in backticks in every doc
    comment.
13. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
14. **No caller changes.** *Check:* `crates/finder/src/imaging/stack/service.rs`,
    `crates/finder/src/imaging/focus/run.rs`,
    `crates/finder/src/imaging/calibration/service.rs`,
    `crates/finder/src/imaging/routes.rs` and `crates/finder/src/mcp/tools.rs` are not in
    the diff.

## Risks

| Risk | Mitigation |
|---|---|
| A real node answers an abort later than 5 s, over a weak link, and is given up on: its exposure runs on, and frames asked for before it ends fail. | Owner decision 1. By reading, a healthy node answers in a round trip. The field check times it. |
| A late abort reaches the node over a real network after the link comes back, and stops a later exposure. | As true on `main`. By reading; owner decision 2's recommendation sends nothing more; the field check looks for it. |
| The rig's node still sends the frame of an exposure begun before it went silent; a real quiet link does not. | The scenarios assert the abort's answer and the frame's state before the exposure would end (§ The rig and a quiet link). |
| 040 landed first and moved cited lines in files both specs write. | Re-anchored at 6d79292b: 040 in `depends_on_landed`, its last writes in `shared_writes`, every citation re-read (Prerequisite 2). |
| The constitution's rule on shared helpers is read to refuse sibling modules' rigs. | Two existing modules do the same; the story escalates rather than copy the rigs (§ Constitution Check). |

## Refinement Record

Drafted on 2026-11-20 by a plate-solver spec session against `main` at fc161a54, and
re-anchored the same day onto 6d79292b once 040 had landed, from
041's and 042's second probe rounds, a probe of the fault and a trial fix on `main` that
day, and a reading of 031's, 033's, 034's, 035's and 039's trios and the constitution.
Writes come from the task text. Shared writes name the merge edge that orders them, and
the story that last wrote the same file on `main`.

The two owner decisions of spec.md are settled (2026-11-20, the recommended answers);
the record's `open_questions` is empty. This record approves nothing:
the owner approvals of spec.md are given by the flip.

```yaml
version: 1
reviewed_revision: 6d79292b1bf5e8a2ba33c17ec0058c031f2115a8
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/imaging_abort_silent.rs
        status: new
        disposition: add
        assertion: "an abort 1 s into a 30 s exposure on a node gone silent answers after 5,000 to 5,100 ms with exactly state no_link; Imaging::ended answers aborted with no file; 1 s later the view is exactly state no_link; ready within 15 s of the node answering again with the frame aborted, no file, status.exposing false and no frames folder; the next frame fails with the busy words and one 25 s later is saved"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/imaging_abort_silent.rs
        status: new
        disposition: add
        assertion: "an abort 1 s into a 30 s exposure on a node that answers answers after 0 to 100 ms with state ready, the frame reads aborted within 1 s with its id, and no frames folder is made"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/imaging_abort_silent.rs
        status: new
        disposition: add
        assertion: "on a node gone silent, a second abort 3 s after a first that waits answers after 0 to 100 ms with exactly state no_link, and the first after 5,000 to 5,100 ms with exactly state no_link"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/imaging_abort_silent.rs
        status: new
        disposition: add
        assertion: "on the stand-in rig whose camera aborts 8,000 ms after it is asked, the abort answers after 5,000 to 5,100 ms with state ready and the frame aborted; a 100 ms frame asked 4 s later fails with the busy words; 30 s later the view's frame is still that frame and no frames folder is made"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/imaging_abort_silent.rs
        status: new
        disposition: add
        assertion: "on the stand-in rig whose camera aborts 3,000 ms after it is asked, the abort answers after 3,000 to 3,100 ms, the frame reads aborted, and a 100 ms frame then is saved"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/imaging_abort_silent.rs
        status: new
        disposition: add
        assertion: "a stack stopped 1 s into its sub on a node gone silent answers after 5,000 to 5,500 ms with state no_link, the stack ended stopped with taken 0, stacked 0 and files exactly its stack.json, that file on disk, and the camera's lease free"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/console/imaging_abort_silent.rs
        status: new
        disposition: add
        assertion: "on a node gone silent, a stack stopped 1 s after an abort that waits answers after 4,000 to 4,500 ms with the stack ended stopped, and the abort after 5,000 to 5,100 ms"
        decision: "spec.md:US1-S7"
      - path: crates/finder/tests/console/imaging_abort_silent.rs
        status: new
        disposition: add
        assertion: "the focus aid stopped 1 s into its 30 s frame on a node gone silent answers after 5,000 to 5,500 ms with exactly state no_link, the aid no longer running and the lease free; once ready again the aid is ended, stopped, with restore_error the scope node is not connected, and the frame aborted"
        decision: "spec.md:US1-S8"
      - path: crates/finder/tests/console/imaging_abort_silent.rs
        status: new
        disposition: add
        assertion: "a set of three darks stopped 1 s into its first dark on a node gone silent answers after 5,000 to 5,500 ms with its set id, the set reads stopped within 1 s more, and the lease is free"
        decision: "spec.md:US1-S9"
      - path: crates/finder/tests/console/imaging_control.rs
        status: existing
        disposition: preserve
        assertion: "an abort of a 30 s exposure on a node that answers ends the frame aborted within 1 s, with no file, and the next frame is 000002.fits"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/stack_loop.rs
        status: existing
        disposition: preserve
        assertion: "a stop abandons a slow solve and aborts a 10 s exposure within 1 s, releasing the lease"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/calibration_darks.rs
        status: existing
        disposition: preserve
        assertion: "a set stopped during a dark, by its route or by an abort, reads stopped within 2 s with the lease released and the frame aborted"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/stack_console.rs
        status: existing
        disposition: preserve
        assertion: "DELETE /api/imaging/stack ends the stack and writes its files, as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/stack_camera.rs
        status: existing
        disposition: preserve
        assertion: "an aborted exposure ends the stack stopped and the stack's lease behaves as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/imaging_focus_aid.rs
        status: existing
        disposition: preserve
        assertion: "the aid's stop and its lost-link ending behave as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/imaging.rs
        status: existing
        disposition: preserve
        assertion: "a dropped link shows no_link within 3 s and ready again within 15 s with the settings kept"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/calibration_console.rs
        status: existing
        disposition: preserve
        assertion: "the console rig and its helpers are used by the new module and not edited"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/imaging.rs
        status: existing
        disposition: preserve
        assertion: "the imaging tools answer as before, an aborted frame among them"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "no test file at the top of crates/finder/tests, the same helper files under support/, and only mod lines in a binary's main.rs"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "an abort the node answers within ABORT_WAIT, a failed call, a frame receiving and an abort with nothing in progress behave as today (spec.md FR-002)"
      - "no caller of Imaging::abort changes, and no route's or view's shape (spec.md FR-003, FR-004)"
      - "the scope node, the scope link's protocol, the mount's Stop and 036's steering are untouched (spec.md FR-004)"
      - "CALL_WAIT, SET_WAIT, CHUNK_WAIT, HEADER_SLACK and the link's heartbeat and silence are unchanged"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console imaging_abort_silent::"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the field check of plan.md § Outside the factory"
    writes:
      - crates/finder/README.md
      - crates/finder/src/imaging/mod.rs
      - crates/finder/tests/console/imaging_abort_silent.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 040-stack-waits-for-steady
        other_story: US11
        disposition: merge_order
        reason: "043 depends_on_landed 040, whose US11 last changed the README at 6d79292b"
      - path: crates/finder/src/imaging/mod.rs
        other_spec: 036-tracking-from-subs
        other_story: US5
        disposition: merge_order
        reason: "043 depends_on_landed 036, whose US5 last changed mod.rs at 6d79292b (72877199)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 040-stack-waits-for-steady
        other_story: US8
        disposition: merge_order
        reason: "043 depends_on_landed 040, whose US8 last changed the module list at 6d79292b (112b3069)"
    open_questions: []
```
