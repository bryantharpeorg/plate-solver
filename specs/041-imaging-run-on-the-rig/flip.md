Not given to builders or the judge. Nothing here is a rule; the trio is.

# Before 041-imaging-run-on-the-rig is flipped to `ready`

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed.** Met: every spec in `depends_on_landed` has landed
   on `main`, each with its last story's merge. `014-goto` at `a43747f7` (US9, #159),
   `015-tracking` at `a02dc35b` (US9, #177), `016-mcp` at `bcf37652` (US6, #186),
   `017-console-stop-confirm` at `c1ed2957` (US4, #206), `030-imaging-camera-link` at
   `49f8dcd2` (US8, #269), `031-imaging-frames-on-console` at `e9687ce9` (US9, #279),
   `032-imaging-solve-and-aim-point` at `670ddcf9` (US7, #288),
   `033-focus-reading-and-aid` at `80f414ff` (US7, #298), `034-dark-and-flat-frames` at
   `002dbef8` (US8, #309), `035-live-stack` at `52785403` (US10, #321),
   `036-tracking-from-subs` at `fc161a54` (US12, #333), `038-track-follows-a-push`,
   whose `push` the `pushed_away` ending reads, at `1c2b427b` (US3, #260), and
   `040-stack-waits-for-steady`, cut from 036 on 2026-11-20, at `6d79292b` (US11, #340).
   `043-imaging-abort-answers-in-time` landed on 2026-11-20 with its one story, US1 at
   `2ae73c66` (#342), and `037-imaging-run` the same day with its five, #344 to #348, the
   last US3 at `581be8c3` (#348).
2. **This trio is re-anchored after `037-imaging-run` and its predecessors land.** Met
   on 2026-11-20 by the re-anchoring, on `main` at `581be8c3`, after 037 landed. Every
   name in plan.md § What 037-imaging-run provides and § What 030 to 036, 038, 040 and 043
   offer the console's parts was re-read in the landed code, and every shared write in
   plan.md § Refinement Record that names another spec names the story that last wrote
   the file on `main` (`git log -1`), as the drafts had named them. None of them offers
   something else than the plan says, and none leaves a part without what it needs. The
   points the console's own parts rest on were read there too, and hold, all but the one
   on M42's transit, which is the simulator's behaviour, measured by probe round 2 on its
   probe code (§ Prerequisites 5) and not read:
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
   - 043 has landed: met, its one story US1 at `2ae73c66` (#342). `Imaging::abort`
     answers within `ABORT_WAIT`, 5 s (`crates/finder/src/imaging/mod.rs:52`), on a
     scope node that has stopped answering (`crates/finder/src/imaging/mod.rs:829`), so
     035's `Stacker::stop`, which awaits it
     (`crates/finder/src/imaging/stack/service.rs:596`), does too, and US8-S6's run ends
     on a lost link (041-R17). The owner settled R17 on 2026-11-20 with 043, which landed
     after 040 and before 037, in place of probe round 2's stand-in, a race of the stop
     against the stack's own `ended` view, which is not folded: the stack part's `stop`
     stays `stacker.stop().await` (plan.md § What 030 to 036, 038, 040 and 043 offer the
     console's parts). 043 covers the camera part's `abort` (US4) too, which awaits the
     same `Imaging::abort` and which the round did not reach on a lost link.
     `depends_on_landed` gained `043-imaging-abort-answers-in-time` at the re-anchoring,
     once 043's trio was on `main`;
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
3. **The lease of the imaging camera exists.** Met on `main`: 031's US9 landed
   (`e9687ce9`, #279), and the re-anchoring of 2026-11-20 read the five things below on
   `581be8c3`, where each holds. The audit round in which this trio was revised had
   written it into the drafts: the plan document's amendment 1 gives 031's imaging
   state one lease that names its holder, and says each later spec takes and gives it
   back and adds no lock of its own. 031's trio gains it as its US9 (the audit's
   finding A1 on 031), and 034 takes it for a set of darks or flats in place of a check
   of its own (the audit's finding A1 on 034), as 033 and 035 do for the focus aid and
   the stack; 033's plan already words the refusal as this trio does. The five things,
   as the landed code has them: the names `Imaging::lease`, `Lease` and `Lease::take`
   (`crates/finder/src/imaging/mod.rs:761`, `crates/finder/src/imaging/mod.rs:899`,
   `crates/finder/src/imaging/mod.rs:937`); that a refusal is a
   409 whose words are `the imaging camera is held by {holder}`; that `Lease::take`
   borrows the lease, so that the kept slot and a take in flight can share one
   `Arc<Lease>`; that
   `Imaging::abort`, `ended`, `view` and `solve` stay open to anyone; and that 033's
   check and aid and 035's stack take the lease themselves, as `the focus aid` and
   `the stack` (`crates/finder/src/imaging/focus/run.rs:28`,
   `crates/finder/src/imaging/stack/service.rs:35`), which is what the hand-over of
   037's owner decision 15 rests on. This
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
5. **A probe build of every story.** Met on 2026-11-20 by probe round 2, for the trio as
   the split of 2026-11-20 left it. Two agents built US4 and US5, then US6, US7 and US8,
   the second starting where the first stopped, on a probe base of 036's, 040's and
   037's round-2 probe builds, tests first and with both gates, from the trio alone:
   history.md was not read. Every story built and passed both gates and wrote only its
   declared files, and every earlier test file passed unedited; workspace nextest ran
   1239 tests at the base and 1299 after US8. The round found 1 blocking and 30 clarity
   defects, 041-R1 to 041-R31, all folded on 2026-11-20 but the anchors of R15 and R31,
   which the re-anchoring of the same day fixed (history.md § Probe round 2 and its fold;
   § Prerequisites 7). Probe round 1,
   on 2026-11-19, had built the trio as it then stood; what this point said of it is in
   the same section. What round 2 ran:
   - each story's diff against its estimate in plan.md § Risks: US4 41,483 B (63.3 %)
     against 40.9 KB, 1.4 % over; US5 31,921 B (48.7 %) against 29.1 KB, 9.7 % over;
     US6 41,188 B (62.8 %) against 37.4 KB, 10.1 % over; US7 14,797 B (22.6 %) against
     14.8 KB, on it; US8 31,488 B (48.0 %) as written against 28.2 KB, 11.7 % over, and
     32,398 B with the round's stand-in for R17. At 1.17, the largest factor of the
     factory builds of 036, 039 and 040, with the cases the fold added as tests, the
     largest estimate is US6's 54,868 B, then US4's 54,052 B, both under owner choice
     14's 60 KB, so no story is cut (plan.md § Risks);
   - each bound, on three runs in paused time, identical on all three:
     - US4-S1, with the true aim point set: the true offset at `stacking` 4.549″, 105.5″
       inside its 110″; the solved offset 0.072″ from it, inside 30″; the tolerance
       exactly 90.0. The first state after `tracking` came 12.1 s after it, which left
       the window of 9.5 s 2.6 s loose; it is now 11.5 s, 0.6 s inside, so that a reach
       of 9 s, whose first frame would come at 11.1 s, fails it too (R1, the owner's
       answer of 2026-11-20);
     - US4-S4: `pushed_away` 5.5 s after the push, 54.5 s inside its 60 s, the one bound
       round 1 did not time; a push that kept moving ended the run 10.4 s after its
       first push, inside the fold's 10 s to 15 s (R2); near the transit, `tracking` held
       for 90.5 s, 60.5 s past its 30 s; at dawn the run stacked from 22 s before
       `sun_up` and ended 0.1 s after the Sun's centre reached −0.83°; the spring-back
       read `moving` between its two pushes;
     - US4-S5: steady 29.9 s after `point`, 870 s inside 900 s, and 12.0 s after
       `repoint`, 12.5 s with US5's step 3, 108 s inside 120 s: every steady wait 2 s
       longer after 036's second fold still holds;
     - US5-S4, with no aim point: one try and 8.52″ by the truth, inside 110″; the aim
       point 0.015 and 0.007 pixels from the true one, inside 2.0; Track asked again
       0.50 s after the re-centre's own Track off, at step 3's second poll, 1.5 s inside
       2 s (R13); the mount moved 0.00″ toward the new aim point before that Track off,
       the model's slip was never marked, no axis moved while Track was off, and every
       `repoint` answered `Ok`. A finder frame every 2 s still centred in one try, at
       5.59″ (R11);
     - US5-S6, at 20 s: one try, 0.98″ by the truth, and 032's last aim reading never
       `moving`. So the fold's `steady_between`, which no probe had run, holds in place
       of round 1's fixed wait, pinned by the tracker's instant alone, by a reach of
       7 s and by a reach that ignores the exposure; US5-S3's two forms of each warning,
       from 036's second fold, are built and pinned;
     - US6-S4: the pause ended at 900.000 s on the limit's own timer, not on a poll, so
       its reads 0.5 s either side hold, and limits of 899 s and 901 s both failed them;
       US6-S9's late resume, 0.999 s after the limit fired, was refused;
     - US6-S7: a good reading 1.000 times its record, 0.10 inside 033's 1.10; clearly out
       at the first step of 1.0, 1.30 times its record;
     - US7-S2 and US7-S8: the limit ended the run 60.0 s after the release, on the run's
       250 ms poll, as US7-S3's ten minutes and US7-S6's did. Reads half a second either
       side fell on polls, and a read at a poll's instant runs before it, so a limit
       0.6 s short survived. The four now read 0.6 s off a whole second, off the poll
       grid, and catch a limit more than half a second off either way (R18); US7-S9's
       read left about 199 s;
     - US8-S1, every part the console's own: `tracking` at 18.5 s, `centring` at 35.0 s
       with one try from 3786.2″, `stacking` at 54.5 s at 2.76″, three subs stacked and
       none dropped, and `count_limit` at 58.5 s, `saved` and with Track on; 036's
       steering read `source` `subs` while it stacked, so it was in the loop;
     - US8-S6: `link_lost` 1.3 s after the node stopped answering, never
       `camera_failed`, on every run, but only with the round's stand-in (R17);
     - US8-S7: the reserve 32,017,096 B, exact; US8-S3's binary passed, and failed when
       given no parts;
   - the seams: every name the trio takes from 031, 032, 033, 035, 036, 038, 040 and 037
     matched the probe's code. One seam defect, R17: 035's `Stacker::stop` awaits 031's
     `Imaging::abort`, which a scope node that has stopped answering never answers, so
     US8-S6's run never finished its ending as written. The owner settled it with 043
     (§ Prerequisites 2), and the round's stand-in is not folded.

   Not run: 040's wait through the console's own stack part, a stack's `camera` ending
   and its words, 033's `no_region`, a failed check's error words, the set point, the
   two hand-over races, a stop with sub files on disk and none stacked, US8-S6's
   `camera_failed` branch, and the camera part's abort on a lost link, which 043 covers.
   A `save_error` can be reached on the rig by a stack folder made read-only (plan.md
   § Risks); whether US8 carries it is the owner's. The fold's new cases are not yet
   committed tests: each ran as a throwaway that passed on the probe's code and failed
   under the mutation it was written for, US5-S8's in US4's module.
6. **The plan document, ADR-0002, the glossary's imaging terms and the research note
   are on `main`.** Met by 2026-11-19: all four are on `main`.
7. **`main` is still as read.** Met on 2026-11-20 by the re-anchoring, on `main` at
   `581be8c3`: every `path:line` in the trio was read there by its content. Of the 44
   the draft held, first read at 49bb210 on 2026-11-15, 23 had not moved and 21 had,
   and those are cited at their lines on `581be8c3`; none was lost. The citations the
   re-anchoring added name 037's landed code (plan.md § What 037-imaging-run provides),
   035's `AppState::stack()` and `crates/finder/Cargo.toml`. If `main` moves before the
   flip, they are read again.
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
