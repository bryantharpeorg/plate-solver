Not given to builders or the judge. Nothing here is a rule; the trio is.

# Before 037-imaging-run is flipped to `ready`

## Prerequisites before this spec is flipped to `ready`

041's and 042's flip.md hold these items for their own stories, under the same numbers.

1. **The earlier specs have landed.** Met for `014-goto`, `015-tracking`, `016-mcp`
   and `017-console-stop-confirm`, and, by 2026-11-19, for `030-imaging-camera-link`,
   `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
   `033-focus-reading-and-aid`, `034-dark-and-flat-frames` and
   `038-track-follows-a-push`, whose `push` the `pushed_away` ending reads. Not met for
   `035-live-stack`, which is flipped and being built, and `036-tracking-from-subs`, a
   draft beside this one.
   Not met for `040-stack-waits-for-steady`, cut from 036 on 2026-11-20.
2. **This trio is re-anchored to its seven predecessors after they land.** Not met. Every
   name in plan.md § What 030 to 036 offer is re-read on `main`, and the last story of each
   spec that writes a shared file replaces the one named in plan.md § Refinement Record. 030, 031,
   032 and 034 were reviewed and fixed before this text was written; 033, 035 and 036
   were drafts then, and have since been reviewed too. Where one of them lands
   offering something else than the table says, the part that calls it changes, and where it offers nothing that a part needs, the
   gap goes to the owner before the flip. Thirteen points are known to need that reading, and a fourteenth is
   closed. Those this spec's own code rests on are here; the rest, which the console's own
   parts rest on, are in 041's flip.md § Prerequisites 2:
   - closed: the rule for a folder's name is written once. 035 offers it as the pure
     function `crate::imaging::stack::service::label`, which its `AppState::target_label`
     calls, and `run::label` calls it too (plan.md § The parts and the words); US12-S2 holds it
     to the worked
     example `NGC 1976` → `NGC-1976`. The re-anchoring re-reads only the function's path;
   - 040's `Steering::waiting` is true only inside 035's loop; `paused_by_tracker` rests
     on it. It is also true while the stack waits for its exposure limit to rise, with
     `steering.pause` `limit` and the tracker steady (040 FR-022), so the run's sentence
     for `paused_by_tracker` names no cause, and the run does not read
     `steering.pause` (plan.md § The run's words);
   - 036's `crate::imaging::steering::FAR_ARCSEC` is public and 180.0, and its steering
     stands back from a stack whose first tracking error is longer (036 FR-014); the two
     forms of the warnings for a target stacked off the centre rest on it. 036's list of
     what this spec reads names the view's `steering.far_arcsec` and not the constant,
     so the re-anchoring reads the constant's path as it reads `label`'s;
3. **The lease of the imaging camera exists.** Met in the drafts. Only 041's camera part
   touches it; the five points the re-anchoring reads are in 041's flip.md
   § Prerequisites 3. US1-S2 and US2-S10 of this spec rest on it through the script.
4. **The owner decisions are settled and the approvals given** (§ Owner decisions
   still open, § Owner approvals). Not met: none of this spec's is settled as a whole;
   on 2026-11-19 the owner's answers to the seventeen choices of the reanalysis
   (history.md § Clarifications) changed 1, 7, 9, 14 and 22, each with a dated note.
   Decision 26,
   the tracker and a newly measured aim point, is not this trio's: it cites 032's owner
   decision 25, which the owner settles in 032's trio, because the behaviour is
   reachable from 032 on, with or without a run (041's plan.md § The console's mount part, "The
   tracker is handed no step"). Met for decision 26: the owner settled 032's decision
   25 on 2026-11-17 with the hold, which 032 writes as its FR-036 and US4-S8, and this
   trio is written for it.
5. **A probe build of every story.** Met on 2026-11-19 by probe round 1, for the trio
   as it then stood. Four agents built all eleven stories from it, tests first and with
   both gates, each starting where the one before stopped (US1 to US3, 041's US4 to US6, 041's US7
   and 041's US8, 042's US9 to US11), on `main`'s 030 to 034 and 038 and on probe builds of 035's
   flipped trio and of 036's folded draft (history.md § Clarifications, the session of
   round 1 probes and the reanalysis). Every story built and passed both gates, two of
   them only with a marked deviation (D13 and D14 below), and each wrote only its
   declared files; every bound but those two held, on three runs each. What the round
   settled:
   - each story's diff against 64 KiB (spec SC-007): US1 61,307 B, 93.5 %; US2 17,566;
     US3 40,205. All were under, and US1 alone was above 85 %. The fold of 2026-11-19
     added code and cases to every story, so plan.md § Risks carries the figures as
     they are estimated now;
   - US3-S6's two runs took 200 ms of their 950 ms;
   - `Imaging` builds its view in one place, `Imaging::view`, which `Imaging::solve`
     answers on `main` too; `run` is set there in every state, and no part was reached
     under the runner's lock (plan.md § Plan traps 23);
   - a console dropped while a run of its own five parts stacks has its mount at rest
     within 0.05 s and lets its session go 2.95 s after the drop, all of that time
     035's stack's (plan.md § Risks). US2-S9 proves the runner's half on the script;

   Not settled, and for the second probe round the owner asked for (history.md
   § Clarifications, the owner's answer 17): everything the fold of 2026-11-19 added,
   none of which has been run (plan.md § Refinement Record, "What probe round 1 and the
   reanalysis changed"); and each story's diff on the folded trio, US12's and US1's
   first, because the owner's split of 2026-11-19 came after the round and no probe has
   built either as it now is (plan.md § Risks).
6. **The plan document, ADR-0002, the glossary's imaging terms and the research note
   are on `main`.** Met by 2026-11-19: all four are on `main`.
7. **`main` is still as read.** Met on 2026-11-15: every `path:line` here was read at
   49bb210, and read again at 49bb210 after the audit, when `main` had not moved; no
   cited line had moved, and `crates/finder/src/targets.rs:39` replaced a citation one
   line above the `struct` it names. It is re-read at the flip, because 030 to 036
   change most of the files cited.
8. **The real mount.** Not met, and outside the factory: a GoTo to a target and Track
   have not run on the real mount (the plan document, § Before any flip, 4). The spec
   can be built before that; the run is not used at the telescope until it is. The
   field checklist's step 3 leaves Track alone on the target for 30 minutes before the
   first run: a night at the telescope logged a garbled reply from the mount every
   few minutes, and if one of those ends Track, no run outlasts it.
9. **No operator-staged files are needed.** Met.

## Owner decisions still open

The owner decisions of this spec's stories, 1, 7, 8, 9, 10, 11, 14, 15, 16, 18, 20, 22,
23 and 25, keep the numbers they had in the twelve-story trio. Each is a design or taste
call the plan document leaves open, but for 11, 15, 22 and 25, whose recommended answer
the plan document now writes into its shared interfaces; they stay here, under their
numbers, for the owner to settle at the flip. None is settled as a whole. On 2026-11-19
the owner's answers to the reanalysis of round 1's probes changed 1, 7, 9, 14 and 22, as
the dated note under each says, and the rest of each waits for the flip. A different
answer is written into the trio and re-validated before the flip, never left to a story.

1. **What an ending does to the mount.** Recommended: nothing. An ending that a guard or
   Stop caused finds the mount as that guard left it, at rest; every other ending (the
   run's own stop, a limit, a camera, solver, save or focus ending) leaves Track on, so
   the target stays in view and Track's own guards still watch it. Instead: every ending
   asks for Stop.
   **Changed by the owner on 2026-11-19:** two endings of a re-centre leave Track off,
   because the run's own part had turned it off and does not turn it on again: a target
   changed while the re-centre had Track off (`target_changed`, 041's owner decision 12), and
   a re-centre that Track refused (`mount_failed`). And a stop of the run that arrives
   while it is still `slewing` also asks for Stop (owner decision 14).
7. **A full disk.** Recommended: one ending, `save_failed`, when any save fails, with the
   system's words. Instead: a free-space check before each sub, which needs a new
   dependency such as `fs4` (MIT OR Apache-2.0) and its own approval.
   **Added by the owner on 2026-11-19:** still no free-space check, but the stack part
   reserves room in the stack's folder when the stack starts and frees it just before it
   stops the stack, so that the ending's own save has room; and the start form shows the
   estimated size of the subs (041's FR-033, FR-035). The room was first answered as a fixed
   64 MiB. **Changed by the owner later that day:** it is as much as the stacked
   picture's files can take for the camera's frame, 15 bytes a pixel and 16 MiB, about
   260 MB at the worked camera's full size, because 64 MiB could not hold a full-size
   `stack.fits` of about 195 MB.
8. **The default limits.** Recommended: a run stops after 7200 s of stacking unless the
   request gives another time, from 1 s to 43,200 s; there is no default number of subs.
   The time is the clock's, from the moment the stack starts, and a stretch in which
   the tracker pauses the stack counts.
   Kept subs of the worked camera are 31 MiB each, so a forgotten run would otherwise
   write until a guard or the disk ended it. Instead: no default limit.
9. **Repeated failures.** Recommended: three imaging frames in a row that the camera
   does not deliver end the run `camera_failed`; three in a row that do not solve end it
   `solver_failing`; ten subs in a row dropped by the stack end it `subs_failing`.
   Instead: other counts; or never ending on dropped subs.
   **Changed by the owner on 2026-11-19:** a failed take is asked again 5 s later, not
   at once, so that a link that drops for a moment can come back (FR-009). And dropped
   subs end the run by time, not by a count: `subs_failing` when 600 s of stacking have
   passed since a sub was last stacked and a sub failed in them. Time in which the
   tracker pauses the stack does not count, and a sub the stack let go because the
   console fell behind (035's `behind` and `unprocessed`) is not a failed sub
   (041's FR-031). The 600 s are a constant, with no request key and no page field.
10. **The target.** Recommended: the run takes the observing session's target. An MCP
    client sets it with the existing `set_target` and then starts. Instead: a `target`
    argument on the route and the tool.
11. **Five endings the plan document's list first lacked, and now names**
    (Clarifications). Recommended: all
    five, each with its own reason word. Instead: fold `mount_failed` and `not_centred`
    into one, and `subs_failing` into `solver_failing`.
14. **Stopping the run does not stop the mount.** Recommended: `Stop imaging run` and
    `stop_imaging_run` end the run and save the stack; the top bar's Stop and the MCP
    `stop` stop the mount and end the run too. A stop of the run that arrives while it
    re-centres is answered once Track has been asked again, within about 5 s, and
    leaves Track on. Instead: the run's stop also asks for Stop.
    **Changed by the owner on 2026-11-19:** a stop of the run that arrives while it is
    still `slewing`, before anything was exposed, also asks for Stop, since the only
    thing the run has done by then is start that motion; in every later state the
    answer above stands. And the wait during a re-centre is about 7 s, not 5: Track's
    own start waits up to 2 s for the axes to rest (FR-005).
15. **The lease is handed over, not held throughout.** The plan document's amendment 1
    makes the run a holder of the one lease of the imaging camera; 033's focus check and
    focus aid and 035's stack, as drafted, each take the lease themselves, as
    `the focus aid` and `the stack`. Recommended: the run holds the lease as
    `the imaging run` while it slews, takes its own imaging frames, solves and centres,
    gives it up just before a focus check and before the stack is started, and takes it
    again after the focus gate. A second taker is refused throughout, with the name of
    whoever holds the camera then, but for the instant of a hand-over; a taker that gets
    in there makes the run's next step fail, and the run says so and ends or pauses.
    Instead: 031's lease lets its holder start 033's and 035's actions as its own, a
    change to both, so that `the imaging run` is named for the whole run and there is no
    such instant.
16. **Stand-in parts in the library.** Recommended: `finder::imaging::run::script`, a
    public scripted stand-in for the run's parts, as `FixedClock` stands in for a clock.
    Nine test modules use it. Instead: a copy in each test module, about 6 KB each
    against the 64 KiB a story may spend; or a new file under
    `crates/finder/tests/support`, which fails the `layout` test, an existing test.
18. **What `run.json` holds.** Recommended: the target, times, settings, states, counts
    and the ending; never the site, the node's address or its token. Instead: the site
    too, for later analysis.
20. **The first imaging frame, and 033's focus check after it.** The plan document's
    order is a first imaging frame, then the focus check; 033's focus check takes its
    own exposures. Recommended: keep the order. The run takes one whole-sensor imaging
    frame first (Clarifications), then runs 033's check, and the imaging solve takes an
    imaging frame of its own: one exposure more than the least possible. Instead: drop
    the first imaging frame and let the focus check's exposures be the run's first,
    which changes the plan document's order; or have 033 offer a verdict for a saved
    imaging frame, so that one imaging frame is taken, measured and solved.
22. **Subs that are not kept.** Recommended: with `keep_subs` false the run removes each
    sub's file and its solve sidecar when the stack has ended, from the list in 035's
    `stack.json`, and removes nothing when the stack could not be saved; the disk still
    holds the subs while the run stacks. Instead: 035's stack removes each sub once it
    is stacked or dropped, an edit to 035.
    **Changed by the owner on 2026-11-19:** the subs are removed only after a saved
    stack of at least one sub that the run itself stopped. A run that stacked nothing,
    or whose stack ended by itself, keeps every sub, and says so when nothing was
    stacked (041's FR-033).
23. **Where `run.json` lies.** Recommended: in the stack's own folder once the run
    stacks, and in `runs/<run id>/` when the run ends before that (Clarifications).
    Instead: 035's `Stacker::start` takes a folder from its caller, and the run makes
    the folder at its start.
25. **Three child modules** (once a departure from the plan document, which first named
    one file, `run.rs`, and now names these three children of it). Recommended:
    `run.rs` holds the machine, `run/script.rs` the scripted
    stand-in, `run/store.rs` the record and `run/console.rs` the console's own parts, so
    that the machine imports nothing of the mount, the camera or the session and no
    story's diff passes 64 KiB. Instead: one file.

## Owner approvals

Flipping this spec to `ready` approves the settled answer to every owner decision above,
and the following. Each is named in the FR or success criterion that makes it.

1. **Three routes**: `POST /api/imaging/run`, `DELETE /api/imaging/run` and
   `POST /api/imaging/run/continue` (FR-013 to FR-015).
2. **The `run` object** in the imaging view of `GET /api/imaging` and `GET /api/session`,
   which the MCP tools that return the session carry too, with its ten state words and
   seventeen ending reasons as stable strings (FR-004, FR-016); and two words this spec
   gives 031's imaging camera, stable from the first run: the purpose `run`, which 031
   writes into the `PURPOSE` card of every imaging frame the run takes itself, and the
   holder's name `the imaging run`, which 031's lease (031 US9) names in every refusal
   while the run holds the camera (041's FR-021). Since the owner's answers of 2026-11-19 the
   `run` object also holds `stacking_since`, `limit_at`, `pause_ends_at`, `ended.saved` and
   `ended.track_on`, and the imaging view the key `unfinished_run` (FR-004, FR-016).
4. **The run's `run.json`**, format `imaging-run/1`, in the stack's folder or in
   `runs/<run id>/` under the imaging folder (FR-017).
5. **A public scripted stand-in**, `finder::imaging::run::script` (FR-008, owner
   decision 16).
8. **New test modules are declared in existing test files** (SC-002). Each is one `mod`
   line, in alphabetical order, in its binary's `main.rs`: `imaging_run`, `imaging_run_routes`, `imaging_run_steps` and `imaging_run_words` in
   `crates/finder/tests/console/main.rs`. No other line of
   those files and no other existing test changes.
9. **Nothing else new.** No dependency, no `Cargo.toml` change, no scope-link message,
   no config key, no console flag, no glossary term and no `unsafe` (SC-003). The
   glossary's **Imaging run** is the plan document's, and **Set point** is 036's.

## Field checks

Nothing of this spec alone is run at the telescope. The run's field checks follow
the checklist of 042's FR-041 once 041 and 042 have landed, and are in their flip.md
§ Field checks; rebuilding the console comes first. GoTo and Track must be proven on the
real mount before any of it (§ Prerequisites 8).

## The checklist tasks.md carried before the split

As it stood, for the twelve-story trio; § Prerequisites is in force.

**Prerequisites** (§ Prerequisites):
- `014-goto`, `015-tracking`, `016-mcp` and `017-console-stop-confirm` have landed on
  `main`: met;
- `030-imaging-camera-link`, `031-imaging-frames-on-console`,
  `032-imaging-solve-and-aim-point`, `033-focus-reading-and-aid`,
  `034-dark-and-flat-frames` and `038-track-follows-a-push` have landed: met by
  2026-11-19;
- `035-live-stack` and `036-tracking-from-subs` have landed, and this trio has been
  re-anchored to all eight, with every name of plan.md § What 030 to 036 offer re-read
  on `main`: not met (§ Prerequisites 1 and 2);
- 031 holds the lease of the imaging camera the plan document's amendment 1 names, as
  its US9, and 033 to 035 take it: met in the drafts, and re-read when 031 lands
  (§ Prerequisites 3);
- the first twenty-five owner decisions in spec.md are settled and its owner approvals
  given: not met; the owner's seventeen answers of 2026-11-19 are folded in, each
  with a dated note (§ Prerequisites 4);
- 032's owner decision 25, the seam between 032 and 015 that decision 26 cites, is
  settled in 032's trio, and this trio is written for its answer: met on 2026-11-17.
  The owner chose the hold (032's FR-036), so 041's FR-022's test counts a held aim point as
  moved and 041's FR-025's wait comes after the re-centre's Track off (plan.md
  § Prerequisites 4);
- every story has been probe-built: met on 2026-11-19 by probe round 1, for the trio as
  it then stood (§ Prerequisites 5);
- a second probe round has built the stories that the fold of 2026-11-19 changed, US12
  and US1 as the owner's split of that day left them among them: not
  met (history.md § Clarifications, the owner's answer 17);
- no operator-staged file is needed.
