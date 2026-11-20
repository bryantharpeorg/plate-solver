---
state: draft
depends_on_landed: ["027-finder-tests-by-system", "030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "034-dark-and-flat-frames", "035-live-stack", "036-tracking-from-subs"]
# Drafted 2026-11-20 against `main` at fc161a54 (fc161a54983b9f69edbc26d63cd376e1dd0e6b7c),
# at the owner's ask the same day ("Yes, draft 043"), from 041's second probe round
# (041-R17), 042's (042-R3) and a probe of that day on `main`: tests of the fault and a
# trial fix of one call, built and run on a branch cut from fc161a54, with both gates'
# Rust commands passing. Every `path:line` here was read at fc161a54. 040, which the
# factory is building, writes two files this spec writes; its trio is not on `main`, so
# it cannot be named here yet (plan.md § Prerequisites 2). State: draft. The seat reviews it and asks the owner before any
# flip; both owner decisions are open. What rests on a run and what on reading is said
# where it is used (plan.md § What was run and what was read).
---

# Feature Specification: An Imaging Abort Answers in Time

**Feature Branch**: `043-imaging-abort-answers-in-time`

**Created**: 2026-11-20

**Status**: Draft

**Input**: 041's second probe round found that a stop on a silent link never answers on
`main` (041-R17), and 042's found the same wait one level up (042-R3). 031's
`Imaging::abort` awaits the scope node's answer to `AbortExposure` with no time limit,
and a node that has stopped answering never gives one. So `POST /api/imaging/abort`
never answers, and nor does any stop that awaits that abort: 035's stack, 033's focus
aid and 034's calibration set. Bound that one call: an abort the node has not answered
in time ends the frame `aborted`, as a failed call already does.

## Clarifications

### Session 2026-11-20 (the probe on `main` at fc161a54)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interface, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it. The probes' notes
  and branches are not in the repository; what they showed is written here.
- Q: What happens today? → A: `Imaging::abort` (`crates/finder/src/imaging/mod.rs:766`)
  takes the frame task out of the state and, for a frame still `exposing`, awaits
  `client.abort()` (`crates/finder/src/imaging/mod.rs:801`), which is
  `ImagingClient::abort` (`crates/finder/src/link.rs:182`): one `AbortExposure` call with
  no time limit. A call that fails ends the frame there; one that answers leaves the
  frame to its task, which ends it when the node's `Aborted` arrives
  (`crates/finder/src/imaging/frames.rs:316`). A call that is never answered is awaited
  for ever. Every stop in the imaging code awaits this abort:
  - `POST /api/imaging/abort` (`crates/finder/src/imaging/routes.rs:228`);
  - 035's `Stacker::stop` (`crates/finder/src/imaging/stack/service.rs:592`), and so
    `DELETE /api/imaging/stack` (`crates/finder/src/imaging/routes.rs:326`) and the MCP
    tool `set_stacking` with `on` false (`crates/finder/src/mcp/tools.rs:915`); the
    stack's taker also aborts a frame it made after a stop
    (`crates/finder/src/imaging/stack/service.rs:789`);
  - 033's `Imaging::focus_stop` (`crates/finder/src/imaging/focus/run.rs:449`), and so
    `DELETE /api/imaging/focus` (`crates/finder/src/imaging/routes.rs:278`) and
    `set_focus_aid` with `on` false (`crates/finder/src/mcp/tools.rs:796`);
  - 034's `Calibration::remove` of a running set
    (`crates/finder/src/imaging/calibration/service.rs:264`), and so
    `DELETE /api/imaging/calibration/{id}`; and the set's own frame after a stop
    (`crates/finder/src/imaging/calibration/service.rs:739`).
- Q: What did the probe see on `main`? → A: On 034's console rig, whose simulated node
  answers nothing once `set_unresponsive(true)` (§ User Scenarios, the silent rig), each
  test sent its call inside a 60 s limit on paused Tokio time, the same on three runs:
  - `POST /api/imaging/abort` 1 s into a 30 s exposure, the node silent: no answer in
    60 s;
  - `DELETE /api/imaging/stack`, `DELETE /api/imaging/focus` and
    `DELETE /api/imaging/calibration/<id>`, each 1 s into its frame's exposure: no
    answer in 60 s. The node's exposure, started before the silence, ran on to its end
    and its frame arrived (plan.md § The rig and a quiet link), so 035's stack read
    `ended` and `stopped`, the aid read `ended` and the set `stopped`, while their
    routes still waited, as 041's probe saw the stack;
  - the first of two aborts: no answer in 60 s; the second answered at once;
  - a stack's stop sent 1 s after an abort that was still waiting: answered after
    28.9 s, when the node's frame arrived;
  - a node whose link answers but whose camera takes 8 s to abort: the abort answered
    after 8 s.
  The mount's Stop is not affected: each StopAll attempt has its own limit
  (`STOP_ATTEMPT`, 200 ms within `STOP_WINDOW`, 2 s: `crates/finder/src/link.rs:34`,
  `crates/finder/src/link.rs:36`, `crates/finder/src/link.rs:384`).
- Q: What is the fix? → A: In `Imaging::abort`'s `Exposing` branch alone, the call is
  bounded by `ABORT_WAIT`, 5 s (owner decision 1). A call not answered by then is
  dropped (owner decision 2) and ends the frame as a failed call does today: the frame
  task is aborted, which drops its stream, and the frame is recorded `aborted`
  (`end_aborted`, `crates/finder/src/imaging/mod.rs:811`). No caller changes. The probe
  built it, and all nine scenarios of US1 passed on three runs, with every other test of
  the workspace unchanged (plan.md § What was run and what was read).
- Q: What does the console show while the link is down, and once it is back? → A: As
  the probe saw it. While the link is down, `GET /api/imaging` and every answer above
  are the view outside `ready`, which holds no frame
  (`crates/finder/src/imaging/mod.rs:487`, `crates/finder/src/imaging/mod.rs:508`):
  for the frame's abort, exactly `{"state":"no_link"}`; for the stack's stop, that and
  the stack. The frame itself is `aborted` with no file, as `Imaging::ended` answers.
  Once the node answers again the camera reads `ready` with that frame `aborted`,
  `status.exposing` false and no file. A frame asked for before the node's own exposure
  has run out fails, `the exposure failed: an exposure is already in progress`; one
  asked for after is saved (US1-S1). The focus aid reads `stopped` with `restore_error`
  `the scope node is not connected` (US1-S8): its settings were not put back, and the
  camera kept the aid's region, as 033 does on a lost link today.
- Q: What becomes of the node's exposure? → A: On the rig, the dropped call never
  reached the camera: the node's exposure ran to its full length, a frame asked for
  before then failed with the camera's `Busy` (the simulated camera's,
  `crates/simulator/src/imaging.rs:776`, in US1-S1; the stand-in's,
  `crates/scope-node/src/imaging/synthetic.rs:180`, in US1-S4) in the words above, and
  one asked for after was saved (US1-S1). By reading, not run: the node then discards that
  exposure's frame, because the console has dropped the stream
  (`crates/scope-node/src/imaging/service.rs:316`); with `save_at_node` it still writes
  its own copy first (`crates/scope-node/src/imaging/service.rs:301`), and its frame
  count moves on (`crates/scope-node/src/imaging/service.rs:297`).
- Q: What does a second abort, or a stack's stop, do while the first abort waits? → A:
  The first abort holds the frame task, so a second finds none to stop and answers the
  view at once, as it does today (US1-S3). A stack's stop then waits for its taker,
  whose frame ends when the first abort's 5 s run out (US1-S7).
- Q: Which other calls to the node does a stop, an abort or a stack's end await? → A:
  Only this one has no limit; every other is bounded and none is in scope (plan.md
  § Other calls on the stop paths).
- Q: Does this change 031's, 033's, 034's or 035's requirements? → A: It changes none of
  their words, and makes them hold on a node that has stopped answering. 031's FR-013
  reads: "`POST /api/imaging/abort` SHALL, under the same-origin rule, abort the
  exposure in progress through `AbortExposure`, or stop receiving a frame in flight, end
  the frame as `aborted`, and answer 200 with the view"
  (`specs/031-imaging-frames-on-console/spec.md:1483`); FR-001 adds when it answers.
  035's FR-018 says `stop()` "SHALL abort the exposure in flight, abandon a solve or a
  measure in flight, and answer once the stack has ended"
  (`specs/035-live-stack/spec.md:2211`), and its FR-032 that
  `DELETE /api/imaging/stack` answers "once it has ended and its files are written"
  (`specs/035-live-stack/spec.md:2295`). 033's FR-028 says `DELETE /api/imaging/focus`
  SHALL "abort the aid's exposure in flight, wait until the aid has ended and answer
  200" (`specs/033-focus-reading-and-aid/spec.md:2165`). Each of them now answers within
  `ABORT_WAIT` and its own work on a silent node.

## Owner decisions

Both are open. Each is a design call with a cost to a node that answers slowly.

1. **How long the console waits for the node's answer to an abort** (FR-001).
   Recommended: 5 s, the console's own limit on the calls its imaging task makes
   (`CALL_WAIT`, `crates/finder/src/imaging/mod.rs:47`), five times the link's own loss
   detection (`SILENCE`, 1 s, `crates/finder/src/link.rs:32`). A stop on a silent node
   then answers in 5 s plus its own work: the page's Stop, 042's `stop_imaging_run` and
   041's run's ending with it. What it costs a node that answers slowly, run on the
   probe with a camera whose abort takes 8 s (US1-S4): the abort answers after 5 s with
   the frame `aborted`, the dropped call never reaches the camera, and the node's
   exposure runs to its full length. A frame asked for before it ends fails with
   `the exposure failed: an exposure is already in progress`; a stack started then
   counts such frames as `frame_failed` and ends `camera` after three in a row
   (`FAILED_IN_A_ROW`, `crates/finder/src/imaging/stack/service.rs:35`; by reading). A
   node that answers within the bound is waited for as today (US1-S5). By reading, not
   run: the INDI adapter answers an abort once its abort switch is written
   (`crates/scope-node/src/imaging/indi/mod.rs:1089`), so a healthy link answers in a
   round trip. Instead: (b) 2 s, the mount's whole stop round (`STOP_WINDOW`), which
   answers sooner and gives up on more slow answers, each with the same cost; or (c)
   10 s, which gives up on fewer and keeps the page's Stop and 042's tool waiting up to
   10 s on a lost link. The scenarios name 5 s; another value changes their numbers
   only.
2. **What becomes of a call given up on** (FR-001). Recommended: drop it, as the trial
   fix does, so the console sends nothing more for that abort. Its cost is decision 1's:
   the node's exposure runs on. Instead: (b) leave the call running in a task of its
   own, so that a late answer still stops the node's exposure. Not tried. By reading,
   that call also reaches a node whose link comes back, and the node's abort names no
   exposure (`AbortExposureRequest` is empty,
   `crates/scope-link/proto/scope_node/v1/scope_node.proto:179`): it stops whatever exposure is then in
   progress, which may be a later frame of the console's own, and a stack started again
   would then end `stopped`.

## Owner approvals

Flipping this spec to `ready` approves the answer to each owner decision above, and the
following. No other existing test, golden or CI file changes.

1. **One line in an existing test file.** US1 adds `mod imaging_abort_silent;` after
   `mod imaging;` (`crates/finder/tests/console/main.rs:18`), as 027's layout asks of
   every new finder test. No other line of that file changes.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command.
- **The silent rig** is built under `#[tokio::test(start_paused = true)]`:
  - a `Simulator` from `calibration_console::sim_rig()`
    (`crates/finder/tests/console/calibration_console.rs:36`), called `sim`, and an
    imaging folder R under a `Scratch` (`crates/finder/tests/support/mod.rs:12`);
  - `calibration_console::console_rig(&sim, &R)`
    (`crates/finder/tests/console/calibration_console.rs:95`), a `SimNode` whose
    controls are the simulator's, served over an in-memory duplex, and a console with
    live solving and imaging;
  - `with_imaging_solver(Arc::new(ScriptedSolver::new()))` called on a clone of the
    rig's `state`, with `stack_loop::ScriptedSolver`
    (`crates/finder/tests/console/stack_loop.rs:64`);
  - `GET /api/imaging` read every 100 ms with `calibration_console::wait_until`
    (`crates/finder/tests/console/calibration_console.rs:128`) until `state` is `ready`;
  - `PUT /api/imaging/settings` with `{"exposure_ms":30000}` answered 200.
  Every request goes through `calibration_console::request`
  (`crates/finder/tests/console/calibration_console.rs:101`) with no `Origin`.
- **The node goes silent** with `sim.set_unresponsive(true)`
  (`crates/simulator/src/controls.rs:117`): from then the node answers no call it is
  sent (`crates/scope-node/src/service.rs:78`), and the console reads `no_link` within
  3 s (031's US1-S6). **The node answers again** with `sim.set_unresponsive(false)`.
- **Exposing** means `GET /api/imaging` read every 100 ms until `frame.state` is
  `exposing`; the frame's `id` is then read. **Taking frame n until it ends** means
  `POST /api/imaging/frame` answered 200 with `frame.id` n, then `GET /api/imaging`
  read every 100 ms until frame n is neither `exposing` nor `receiving`.
- **A timed request** is sent inside `tokio::time::timeout(Duration::from_secs(60), …)`,
  and its time is the paused Tokio time from sending it to its answer. It must answer
  200; "answers after a to b ms" means its time is in that closed range.
- **The slow rig** is `stack_loop::stand_in_rig`
  (`crates/finder/tests/console/stack_loop.rs:268`) given `ScriptedSolver::new()` and a
  camera of the test's own, `SlowAbort`: a `SyntheticImagingCamera` on a node clock that
  reads `VirtualClock::new(START)`'s `node_now()`, with `START` from
  `crates/finder/tests/support/live.rs:34`, passing every call through but `abort`,
  which first sleeps `late` and then calls the stand-in's abort. Its exposure is set to
  30 s, a frame is taken, exposing, and 1 s passes.
- **Oracles.** The routes' answers, `Imaging::ended`, `Imaging::holder`,
  `Imaging::focus_running`, the stack's file on disk, the words in these scenarios, and
  paused Tokio time. No expected value comes from the code under test: 5,000 ms is the
  scenarios' own number. The abort's tolerance is 100 ms; a stop that also writes files
  or ends a task takes 500 ms, since paused time is not measured across work on the
  blocking pool.
- **The busy words** are `the exposure failed: an exposure is already in progress`.

### User Story 1 - An abort answers whether or not the node does, and so does every stop that awaits it (Priority: P1)

As the observer, I press Abort or Stop while the scope node has dropped off the network,
and the console answers within seconds and says the frame or the stack has ended; once
the node is back, the camera works again.

**Why this priority**: Today one lost link mid-exposure leaves the page's Stop pressed
for good, and the stack, the focus aid, 041's run and 042's stop tool with it, until the
console is restarted.

**Independent Test**: `crates/finder/tests/console/imaging_abort_silent.rs`, the module
`imaging_abort_silent` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_abort_silent::`.

**Acceptance Scenarios**:

1. **Given** the silent rig with a frame taken, exposing, 1 s passed and then the node
   gone silent, **When** a committed test sends a timed `POST /api/imaging/abort`, reads
   `GET /api/imaging` 1 s after its answer, lets the node answer again, reads the view
   until `state` is `ready`, applies `{"exposure_ms":100}`, takes frame id+1 until it
   ends, waits 25 s and takes frame id+2 until it ends, **Then** it asserts that the
   abort answers after 5,000 to 5,100 ms with exactly `{"state":"no_link"}`; that
   `Imaging::ended(id)` answers `state` `aborted` and `file` `None`; that the later read
   is exactly `{"state":"no_link"}`; that `ready` comes within 15 s of the node
   answering again, with `frame.id` id, `frame.state` `aborted`, no `frame.file`,
   `status.exposing` false and no folder `frames` under R; that frame id+1 ends `failed`
   with `error` the busy words; and that frame id+2 ends `saved`.
2. **Given** the silent rig with a frame taken, exposing and 1 s passed, the node
   answering, **When** a committed test sends a timed `POST /api/imaging/abort` and then
   reads `GET /api/imaging` every 100 ms until `frame.state` is `aborted`, **Then** it
   asserts that the abort answers after 0 to 100 ms with `state` `ready`, that
   `frame.state` is `aborted` within 1 s of the answer with `frame.id` id, and that R
   holds no folder `frames`.
3. **Given** the silent rig with a frame taken and exposing, the node then gone silent,
   and a timed `POST /api/imaging/abort` sent from a spawned task, **When** a committed
   test sends a second timed `POST /api/imaging/abort` 1 s later, **Then** it asserts
   that the second answers after 0 to 100 ms with exactly `{"state":"no_link"}`, and
   that the first answers after 5,000 to 5,100 ms with exactly `{"state":"no_link"}`.
4. **Given** the slow rig with `late` 8,000 ms, **When** a committed test sends a timed
   `POST /api/imaging/abort`, waits 1 s, applies `{"exposure_ms":100}` and takes frame
   id+1 until it ends, **Then** it asserts that the abort answers after 5,000 to
   5,100 ms with `state` `ready`, `frame.id` id and `frame.state` `aborted`, and that
   frame id+1 ends `failed` with `error` the busy words.
5. **Given** the slow rig with `late` 3,000 ms, **When** a committed test sends a timed
   `POST /api/imaging/abort`, reads `GET /api/imaging` every 100 ms until `frame.state`
   is `aborted`, applies `{"exposure_ms":100}` and takes frame id+1 until it ends,
   **Then** it asserts that the abort answers after 3,000 to 3,100 ms, that
   `frame.state` reads `aborted` within 60 s of it, and that frame id+1 ends `saved`.
6. **Given** the silent rig with `POST /api/imaging/stack` and `{}` answered 200, its
   `stack.id` read as S, its sub exposing, 1 s passed and then the node gone silent,
   **When** a committed test sends a timed `DELETE /api/imaging/stack`, **Then** it
   asserts that the stop answers after 5,000 to 5,500 ms with `state` `no_link`,
   `stack.state` `ended`, `stack.ended_reason` `stopped`, `stack.taken` 0,
   `stack.stacked` 0 and `stack.files` exactly `{"json":"runs/S/stack.json"}`; that
   `runs/S/stack.json` is a file under R; and that `Imaging::holder()` is `None`.
7. **Given** the silent rig with `POST /api/imaging/stack` and `{}` answered 200, its
   sub exposing, the node then gone silent, and a timed `POST /api/imaging/abort` sent
   from a spawned task, **When** a committed test sends a timed
   `DELETE /api/imaging/stack` 1 s later, **Then** it asserts that the stop answers
   after 4,000 to 4,500 ms with `stack.state` `ended` and `stack.ended_reason`
   `stopped`, and that the abort answers after 5,000 to 5,100 ms.
8. **Given** the silent rig with `POST /api/imaging/focus` and
   `{"tile":{"col":1,"row":1},"exposure_ms":30000}` answered 200, its frame exposing,
   1 s passed and then the node gone silent, **When** a committed test sends a timed
   `DELETE /api/imaging/focus`, lets the node answer again and reads the view until
   `state` is `ready`, **Then** it asserts that the stop answers after 5,000 to
   5,500 ms with exactly `{"state":"no_link"}`; that at that answer
   `Imaging::focus_running()` is false and `Imaging::holder()` is `None`; and that once
   `ready` the view has `focus.aid.state` `ended`, `focus.aid.ended_reason` `stopped`,
   `focus.aid.restore_error` `the scope node is not connected` and `frame.state`
   `aborted`.
9. **Given** the silent rig built after `sim.set_light_path` with `cover` true and the
   rest of `LightPath::default()`, with `POST /api/imaging/calibration/darks` and
   `{"count":3}` answered 200, its `set.id` read as D, `GET /api/imaging` read until
   `calibration.set.state` is `frames` and `status.exposing` is true, 1 s passed and
   then the node gone silent, **When** a committed test sends a timed
   `DELETE /api/imaging/calibration/D` and then reads `GET /api/imaging/calibration`
   every 100 ms until `set.state` is `stopped`, **Then** it asserts that the stop
   answers after 5,000 to 5,500 ms with `set.id` D, that `set.state` reads `stopped`
   within 1 s of that answer, and that `Imaging::holder()` is then `None`.

### Edge Cases

- **The node answers again in the instant the console gives up.** The probe saw it
  once, with the node made to answer again at the abort's answer: the dropped call was
  still waiting on the node, the node carried it out, its camera was free at once and
  the next frame was saved. US1-S1 keeps the node silent for 1 s after the answer, which
  the probe ran three times with the busy words each time.
- **A node that answers the abort and then never sends `Aborted`.** The frame is left
  to its task, as today, and waits out the header's limit, twice the exposure plus
  150 s (`crates/finder/src/imaging/frames.rs:32`). By reading; not run, and out of
  scope.
- **A real link that has gone quiet.** The rig's node still finishes an exposure begun
  before it went silent and sends its frame (plan.md § The rig and a quiet link). Over a
  real quiet link the frame's stream is silent too, so on `main` the frame would wait
  out the header's limit; with this spec it ends `aborted` after 5 s either way. By
  reading.
- **An abort that reaches the node late over a real network.** The call's bytes may
  already be on their way when the link goes quiet, and reach the node when it comes
  back, after the console has given up; the node then stops whatever exposure is in
  progress. This is as true on `main`, where the console waits for the answer. By
  reading; the field check looks for it (plan.md § Outside the factory).
- **The aid stopped between frames in the first second of silence.** With no frame in
  flight there is nothing to abort, and the camera still reads `ready`, so the aid's
  settings put back go to the node, and `DELETE /api/imaging/focus` waits up to
  `SET_WAIT`, 30 s (`crates/finder/src/imaging/mod.rs:663`), for that refusal before the
  aid ends. Bounded, by reading, and out of scope. US1-S8 stops the aid mid-frame, and
  the camera reads `no_link` by the time the aid ends.
- **A frame `receiving`** is stopped at the console at once, as today
  (`crates/finder/src/imaging/mod.rs:792`).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `Imaging::abort` (`crates/finder/src/imaging/mod.rs:766`), for a frame
  `exposing`, SHALL wait for the node's answer to `AbortExposure` no longer than
  `ABORT_WAIT`, a public constant of `crates/finder/src/imaging/mod.rs` of 5 s (owner
  decision 1). A call not answered by then SHALL be dropped (owner decision 2) and SHALL
  end the frame as a failed call does today: the frame task aborted, its stream dropped
  with it, and the frame `aborted` and the last ended frame. It SHALL then answer the
  view (US1-S1, US1-S3, US1-S4).
- **FR-002**: A call the node answers OK within `ABORT_WAIT` SHALL leave the frame to
  its task, which ends it `aborted` when the node's `Aborted` arrives, and a call that
  fails SHALL end it at once, both as today (031's FR-013). An abort of a frame
  `receiving`, or with nothing in progress, SHALL be as today (US1-S2, US1-S5).
- **FR-003**: Every caller of `Imaging::abort` SHALL be unchanged: the abort route,
  `Stacker::stop` and the stack's taker, `Imaging::focus_stop`, `Calibration::remove`
  and the calibration set's frame, and through them `DELETE /api/imaging/stack`,
  `DELETE /api/imaging/focus`, `DELETE /api/imaging/calibration/{id}` and the MCP tools
  `set_stacking` and `set_focus_aid` with `on` false. On a node that has stopped
  answering each SHALL answer within `ABORT_WAIT` and its own work, as its spec says it
  answers (US1-S6 to US1-S9).
- **FR-004**: Nothing else SHALL change: the scope node, the scope link's protocol
  (`crates/scope-link/proto/scope_node/v1/scope_node.proto`), every route's and view's
  shape, the mount's Stop, 036's and 040's steering, the frame task's own limits
  (`crates/finder/src/imaging/frames.rs:24`, `crates/finder/src/imaging/frames.rs:27`),
  `CALL_WAIT`, `SET_WAIT` and the link's heartbeat and silence. No file under
  `crates/finder/src` but `crates/finder/src/imaging/mod.rs` SHALL be written.
- **FR-005**: `crates/finder/README.md` SHALL say, in the row of
  `POST /api/imaging/abort` (`crates/finder/README.md:231`) and in the paragraph on the
  abort (`crates/finder/README.md:662`), that a node that does not answer the abort
  within 5 s is given up on and the frame ends `aborted` then, that the node's exposure
  may run on, and that a frame asked for before it ends fails with the busy words; and
  in the row of `DELETE /api/imaging/stack` (`crates/finder/README.md:241`) that on a
  node that has stopped answering the stop answers after 5 s.

### Key Entities

- **`ABORT_WAIT`**: how long the console waits for the node's answer to an abort, 5 s.
- **An abort given up on**: a call to `AbortExposure` the node did not answer within
  `ABORT_WAIT`, dropped, its frame ended `aborted` by the console.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited, but for the
  one `mod` line of owner approval 1. Every existing test passes unchanged (plan.md
  § Shapes that existing tests pin).
- **SC-003**: On a node that has stopped answering, an abort and every stop that awaits
  it answer within 5.5 s (US1-S1, US1-S3, US1-S4, US1-S6 to US1-S9), where today none
  answers within 60 s.
- **SC-004**: On a node that answers within 5 s, an abort is answered as it is today
  (US1-S2, US1-S5).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed, and
  so has 040, which is named there once its trio is on `main`. Every citation here was
  read at fc161a54 (plan.md § Prerequisites 2).
- **The rig stands for a node that has stopped answering.** `set_unresponsive(true)`
  holds every call the node is sent, as a node off the network does, but lets an
  exposure already started run on and send its frame (plan.md § The rig and a quiet
  link).
- **How long a real node takes to answer an abort over the link is not known.** The
  field check measures it.
- **Out of scope:**
  - any change to the scope node, such as naming the exposure an abort is for;
  - telling the caller that the node did not confirm the abort: the answer's shape is
    unchanged (FR-004);
  - a frame whose `Aborted` never comes after the node answered the abort (Edge Cases);
  - the focus aid's settings left as the aid set them after a lost link, which is 033's
    behaviour today;
  - 041's and 042's trios, which 043 does not edit (plan.md § What this gives later
    specs).
- **Outside the factory:** the field check in plan.md § Outside the factory, by the
  owner.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005]
```

One story: the bound, its tests and the README.
