Not given to builders or the judge. Nothing here is a rule; the trio is.

# Before 041-imaging-run-on-the-rig is flipped to `ready`

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed.** Met for `014-goto`, `015-tracking`, `016-mcp`
   and `017-console-stop-confirm`, and, by 2026-11-19, for `030-imaging-camera-link`,
   `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
   `033-focus-reading-and-aid`, `034-dark-and-flat-frames` and
   `038-track-follows-a-push`, whose `push` the `pushed_away` ending reads. Not met for
   `035-live-stack`, which is flipped and being built, and `036-tracking-from-subs`, a
   draft beside this one.
   Not met for `040-stack-waits-for-steady`, cut from 036 on 2026-11-20,
   nor for `037-imaging-run`.
2. **This trio is re-anchored after `037-imaging-run` and its predecessors land.** Not
   met. Every name in plan.md § What 037-imaging-run provides and § What 030 to 036, 038
   and 040 offer the console's parts is re-read on `main`, and the last story of each
   spec that writes a shared file replaces the one named in plan.md § Refinement Record. Where
   one of them lands offering something else than the plan says, the part that calls it
   changes, and where it offers nothing that a part needs, the gap goes to the owner
   before the flip. The points known to need that reading, which the console's own
   parts rest on:
   - 033 writes its focus check as taking its own exposures and restoring the settings
     afterwards; the run's first imaging frame and the imaging frame for the solve rest
     on that (037's owner decision 20);
   - 035's `Stacker::start` names its own folder, keeps every sub, and has no limit;
     the record's place and the removal of subs rest on that (037's owner decisions 22
     and 23);
   - 035's view ends with `stopped` or `camera` and a `save_error`; if it gains other
     reasons, `StackProgress::failed` sorts them;
   - 035's `Stacker::stop` is safe on a stack that has already ended: the machine asks
     the stack part to stop at every ending after a start, whoever ended the stack
     (037's plan.md § The machine, Ending), and the part calls `Stacker::stop` only while its own
     stack still reads `stacking`, so the point matters for a stack that ends between
     that read and the call (plan.md § The console's stack part);
   - 035 writes, when a stack ends, `stack.fits` as three planes of 32-bit reals,
     `stack.png` as an 8-bit RGB picture of the stack's full size, and `stack.json`
     again whole; the stack part's reserve is sized to those three (plan.md § The console's
     stack part);
   - 036's `Steered::steady`, read through `Session::steered`, is false while a tracker
     settles after a start; the run's
     first exposure waits on it;
   - 036's tracker is not steady for at least 60 s around M42's transit on the
     simulator; US4-S4's unsteady bullet rests on it;
   - 040's `Steered::steady_between` is false from a tracker's start until 2 s after its
     first rates, across every take-up and keyhole pass with the 2 s that follow each,
     across a measurement, while 038's push reads `moving`, and for a turn that finds an
     axis stopped, over a window of console Unix seconds; a window that begins at the
     instant such a time ended is not touched by it (040 FR-021). 040's US10-S3 pins the
     window the mount part asks, ending now and 10 s long: false asked 6 s after a
     tracker's start and true asked 14 s after it. The mount part's `steady` and the
     repeat of an unsteady focus check rest on it;
   - 035's `StackView::started_at` is set once, at the start that made the stack, and
     035's `start` on a running stack answers `Ok` with that stack's view; the stack
     part tells its own stack from another's by it;
   - 035's `dropped` holds `behind` and `unprocessed` for subs the console did not
     get to, and no other reason means that; the rule for subs that keep failing
     leaves those two out;
   - 038's `push` reads `moving` from the first live pointing more than 5′ off and
     `following` after 5 s of stillness, and is absent otherwise; `pushed_away` and
     its 10 s rest on it. 040's tracker reads not steady while a push is `moving` (040
     FR-021, 040's US6-S6);
   - 036's `Steering::set_set_point` lets go a tracker a stack holds when the set point
     changes (036 FR-014); the stack part calls it before the stack's start, where it
     changes nothing a stack holds;
   - 032's task sets a measured aim point in the session before it shows a solve as
     `solved`, and sets it only when `should_apply` says so; while Track is on it
     holds it back instead, `Session::held_aim` answers it, and the first live solve
     the session records with Track off applies it before that pointing is taken
     (032's FR-036). `aim_moved` and the wait of `repoint`'s step 3 rest on all of it
     (plan.md § The console's camera and solver parts, plan.md § The console's mount part).
3. **The lease of the imaging camera exists.** Met in the drafts, by the audit round in
   which this trio was revised. The plan document's amendment 1 gives 031's imaging
   state one lease that names its holder, and says each later spec takes and gives it
   back and adds no lock of its own. 031's trio gains it as its US9 (the audit's
   finding A1 on 031), and 034 takes it for a set of darks or flats in place of a check
   of its own (the audit's finding A1 on 034), as 033 and 035 do for the focus aid and
   the stack; 033's plan already words the refusal as this trio does. 031's plan.md
   § The camera lease now writes 042's US9 out, and the five things below hold there; this
   trio is read again against it when 031 lands, for the same five
   things: the names `Imaging::lease`, `Lease` and `Lease::take`; that a refusal is a
   409 whose words are `the imaging camera is held by {holder}`; that `Lease::take`
   borrows the lease, so that the kept slot and a take in flight can share one
   `Arc<Lease>`; that
   `Imaging::abort`, `ended`, `view` and `solve` stay open to anyone; and that 033's
   check and aid and 035's stack take the lease themselves, as `the focus aid` and
   `the stack`, which is what the hand-over of 037's owner decision 15 rests on. This
   trio does not need 031 to put its lease check before its own check for an imaging
   frame in flight: the camera part asks for the lease first (plan.md § The console's camera and
   solver parts). 037's US1-S2, 037's US2-S10, US4-S2, US4-S4, US4-S6, US6-S7 and
   US8-S8 are the scenarios that rest on the lease. Two state its refusal word for word:
   `the imaging camera is held by the imaging run` (US4-S4) and
   `the imaging camera is held by the focus aid` (US6-S7).
4. **The owner decisions are settled and the approvals given** (§ Owner decisions
   still open, § Owner approvals). Not met: none of this spec's is settled as a whole;
   on 2026-11-19 the owner's answers changed 3, 4, 12 and 27, each with a dated note.
   Met for 037's decision 26, which 032's owner decision 25, settled on 2026-11-17 with
   the hold, answers (037's flip.md § Prerequisites 4). Every aim reading of a run is taken while Track is on and
   held back, so FR-022's test counts a held aim point as moved and FR-025's wait comes
   after the re-centre's Track off, whose first live pointing puts it in force; US5-S4,
   SC-004 and the whole runs of US8-S1, US8-S2 and US8-S8 rest on both. This trio is
   re-validated with 032's before the flip.
5. **A probe build of every story.** Met on 2026-11-19 by probe round 1, for the trio
   as it then stood, as 037's flip.md § Prerequisites 5 tells. What the round settled for
   this spec's stories:
   - each story's diff against 64 KiB (spec SC-007): US4 32,441 B; US5 23,300; US6
     31,886; US7 11,298; US8 20,250. All were under. The fold of 2026-11-19 added code
     and cases to every story, so plan.md § Risks carries the figures as they are
     estimated now;
   - US4-S1 leaves the target 4.43″ from the centre by the truth with the true aim
     point set, against its 110″, and its solved offset is 0.07″ from the truth,
     against 30″;
   - US5-S4 centres in one try, 8.67″ by the truth, with the aim point 0.017 and 0.009
     pixels from the true one, against 2.0. It did so only once the mount part read
     steady no sooner than 032's fit reach after Track first worked: an imaging
     exposure started 8.2 s after `tracking` read `moving` and took no aim reading,
     and one started 9.2 s after it read `measured` (D14, blocking). The round built
     that as a fixed wait of 10 s; the fold wrote `steady_between` in its place (the
     owner's answer 2), which no probe has run;
   - the simulator's mount did not move toward the newly measured aim point between
     the first imaging solve and the re-centre's Track off (0.00″), the pointing
     model's slip was not marked then, and the first live pointing after that Track
     off came 0.30 s later, against the 5 s of `repoint`'s step 3;
   - the simulated axes are at rest by the time a re-centre's Track asks GoTo, and
     Track itself waits up to 2 s for that; GoTo did not refuse (plan.md § Risks);
   - US6-S4's pause ended at 900.000 s and US7-S2's limit at 60.000 s, each on a poll.
     Their reads were a second either side, and a limit a second short survived
     them, so both now read half a second before and after (D22, D25);
   - the run rig syncs, slews and solves at dawn: US4-S4's run stacked for 31 s before
     `sun_up`;
   - US8-S1 reached three stacked subs on the simulator with 036's steering in the
     loop, none dropped, on all three runs;
   - one more defect blocked a story as written: US4-S5 read rest the instant after a
     Stop, while the axes still braked (D13: it now reads from 2 s after).

   Not settled, and for the second probe round the owner asked for (037's history.md
   § Clarifications, the owner's answer 17): everything the fold of 2026-11-19 added,
   none of which has been run; each story's diff on the folded trio; what 036's own fold
   changed underneath this trio the same day (037's history.md, the Refinement Record's
   prose, "036's second fold"): the two forms of the warnings in US5-S3, and every wait
   for a steady tracker 2 s longer on the simulator, inside which US4-S5's 900 s and
   120 s must still hold; and the margin of US4-S4's "within 60 s" for `pushed_away`,
   the one bound round 1 did not time.
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

The owner decisions of this spec's stories, 2, 3, 4, 5, 6, 12, 21, 24 and 27, keep the
numbers they had in the twelve-story trio. Each is a design or taste call the plan
document leaves open. None is settled as a whole. On 2026-11-19 the owner's answers to
the reanalysis of round 1's probes changed 3, 4, 12 and 27, as the dated note under each
says, and the rest of each waits for the flip. Decision 26 is 037's, settled (037's
history.md). A different answer is written into the trio and re-validated before the
flip, never left to a story.

2. **When the target counts as centred.** Recommended: within the larger of 1/20 of the
   imaging frame's shorter side on the sky and 90″ of the sensor's centre: 90″ for the
   worked telescope, whose imaging frame is 0.4997° high. The floor is one and a half times
   GoTo's own 60″, below which a re-centre cannot reliably land. Instead: a fixed 2′; or
   1/10 of the shorter side.
3. **How many centring tries.** Recommended: three re-centres, so at most four solves.
   Instead: five, as GoTo's own corrections.
   **Changed by the owner on 2026-11-19:** a solve outside the tolerance that measured
   no new aim point asks for no re-centre and is not a try, so a run can make more
   than four solves; four such solves in a row end the wait: the run then stacks with a
   warning when the
   target is near the centre (owner decision 4), and otherwise ends `not_centred` with a
   sentence of its own (FR-024).
4. **A target that cannot be centred.** Recommended: the run ends `not_centred`.
   Instead: stack anyway with a warning whenever the target is inside the imaging frame.
   **Changed by the owner on 2026-11-19** to a part of the other answer: after the
   three tries a target within a quarter of the imaging frame's shorter side of the
   sensor's centre is stacked with a warning. Within 180″ of the centre 036's steering
   brings it in over the following subs, at most 30″ on the sky a gap, and far more
   slowly where an axis would have to back up; beyond 180″ 036 does not steer that stack
   at all, and the target is stacked where it is. The warning says which (036 FR-008,
   037's FR-014; corrected the same day, after 036's own fold: this note first said that the
   steering brings every such target in within a few subs). Only beyond the quarter does
   the run end `not_centred` (FR-024). The same holds for a run
   whose four solves in a row measured no new aim point (owner decision 3): near the
   centre it is stacked with a warning of its own, and it ends `not_centred` only beyond
   the quarter.
5. **Going on from a focus pause.** Recommended: one action with a switch. "Check focus
   again" repeats the focus check, pausing again if it is still clearly out; "Go anyway"
   goes on unchecked. Instead: one action that always goes on unchecked; or the run
   resuming by itself when the focus aid's reading turns good.
6. **How long a focus pause may last.** Recommended: 15 minutes, then the run ends
   `focus_timeout` with Track still on. Instead: no limit; or 5 minutes.
12. **A changed target.** Recommended: when the session's target is changed under a run,
    the run ends `target_changed` and leaves Track as it is. Instead: refuse a target
    edit while a run is on.
    **Changed by the owner on 2026-11-19:** a target changed while a re-centre has
    Track off leaves Track off: the run does not ask Track for the new target, and
    ends `target_changed` (FR-023).
21. **No focus record yet.** Recommended: the verdict `no_record` pauses the run and
    opens the focus aid, as clearly out does, with its own words; using the focus aid
    makes the record, and "Go anyway" goes on. Instead: warn and go on.
24. **A region of interest.** Recommended: a run is refused while the imaging camera's
    settings hold a region of interest, with words that say to clear it: 032 takes an
    aim reading from whole-sensor imaging frames only, and the centre of a region is not
    the set point. Instead: measure centring through the solve's own map at the sensor's
    centre and let such a run go on without aim readings.
27. **A push by hand during a run.** Recommended: when the session's Track view
    carries 038's `push`, the telescope moving by hand or Track following a push, the
    run ends `pushed_away`, saves the stack and asks nothing of the mount, so Track goes
    on following where the observer left it. Subs taken across a push are smeared, and
    the target may no longer be in the frame. Instead: pause the run until the observer
    goes back to the target with Track, then centre again.
    **Changed by the owner on 2026-11-19:** the run ends `pushed_away` only when the
    push has settled somewhere else (038's `following`) or has lasted 10 s (`moving`
    all that time). A bump that springs back sooner, which 038 itself forgives, does
    not end the run (FR-023).

## Owner approvals

Flipping this spec to `ready` approves the settled answer to every owner decision above,
and the following. Each is named in the FR or success criterion that makes it.

8. **New test modules are declared in existing test files** (SC-002). Each is one `mod`
   line, in alphabetical order, in its binary's `main.rs`: `imaging_run_centring`, `imaging_run_focus` and `imaging_run_limits` in
   `crates/finder/tests/console/main.rs`; `imaging_run`, `imaging_run_centring` and
   `imaging_run_stack` in `crates/finder/tests/motion/main.rs`; and `imaging_run` in
   `crates/finder/tests/cli/main.rs`. No other line of
   those files and no other existing test changes.
9. **Nothing else new.** No dependency, no `Cargo.toml` change, no scope-link message,
   no config key, no console flag, no glossary term and no `unsafe` (SC-003). The
   glossary's **Imaging run** is the plan document's, and **Set point** is 036's.

The purpose `run` and the holder's name `the imaging run` this spec's FR-021
gives 031's imaging camera are approved by 037's owner approval 2.

## Field checks

At the telescope, by the checklist of 042's FR-041, once 042 has landed: how many
centring tries a real night needs and how far off the first solve finds the target;
whether 90″ suits the real mount; the real focus pause, with the focuser turned by
hand; a run through a Stop; that the real mount does not move toward a newly measured
aim point between a first imaging solve and the re-centre's Track off (032's owner
decision 25); and whether the corner stars of the run's own imaging frames trail at
the exposure setting used for subs. GoTo and Track must be proven on the real mount
before any of it (§ Prerequisites 8).
