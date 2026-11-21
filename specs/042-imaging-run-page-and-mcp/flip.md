Not given to builders or the judge. Nothing here is a rule; the trio is.

# Before 042-imaging-run-page-and-mcp is flipped to `ready`

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
   `2ae73c66` (#342); `037-imaging-run` with its five, #344 to #348, the last US3 at
   `581be8c3` (#348); and `041-imaging-run-on-the-rig` with its five, #350 to #354, the
   last US8 at `61b54a47` (#354).
2. **This trio is re-anchored after `037-imaging-run` and `041-imaging-run-on-the-rig`
   land.** Met on 2026-11-20 by the re-anchoring, on `main` at `61b54a47`, after 041
   landed. Every name in plan.md § What 037-imaging-run provides and § What
   041-imaging-run-on-the-rig provides was re-read in the landed code, and every shared
   write in plan.md § Refinement Record that names another spec names the story that last
   wrote the file on `main` (`git log -1`): the README's is 043's US1 (`2ae73c66`, #342),
   which wrote it after 040's US11, and the others are as the drafts named them. Where
   037 or 041 landed offering something else than those sections said, the sections now
   say what landed. One of them reached a story's text: `POST /api/imaging/run` reads its
   body as a JSON object and then as a `RunRequest` with `serde_json::from_value`
   (`run_body`), so its 400 for an unknown key is serde's words alone, where the probe's
   route took a `Json<RunRequest>` and answered axum's
   `Failed to deserialize the JSON body into the target type` before them. plan.md
   § The MCP tools had `start_imaging_run` parse its arguments the way `set_target`
   parses its edit, which gives axum's words; it now parses them as `run_body` does, so
   the tool still answers the route's words, as FR-037 and US10-S3 ask, and neither
   changed. The point known to need that reading:
   - 043 has landed: met, its one story US1 at `2ae73c66` (#342). `Imaging::abort`
     answers within `ABORT_WAIT`, 5 s (`crates/finder/src/imaging/mod.rs:52`), on a
     scope node that has stopped answering (`crates/finder/src/imaging/mod.rs:829`), so
     041's run's ending finishes on a lost link, and `stop_imaging_run` and
     `DELETE /api/imaging/run` answer (042-R3). Both wait on `Runner::stop`, which waits
     for the run's ending, which awaits 041's stack part's `stop`
     (`crates/finder/src/imaging/run/console.rs:718`), which awaits 035's `Stacker::stop`
     and so `Imaging::abort` (`crates/finder/src/imaging/stack/service.rs:596`); probe
     round 2 found that, with 041's stop as its trio writes it, the tool did not answer
     within 60 s of virtual time on a lost link, and answered 1 s after the node went
     quiet with 041's round-2 stand-in. The owner settled 041-R17, and 042-R3 with it, on
     2026-11-20 with 043, so 041's stack part keeps `stacker.stop().await`
     (`crates/finder/src/imaging/run/console.rs:734`) and no stand-in is folded here
     either. `depends_on_landed` gained `043-imaging-abort-answers-in-time` at the
     re-anchoring, once 043's trio was on `main`.
3. **The lease of the imaging camera exists.** Nothing of this spec touches it
   (041's flip.md § Prerequisites 3).
4. **The owner decisions are settled and the approvals given** (§ Owner decisions,
   § Owner approvals). Met on 2026-11-20: the decisions are settled as § Owner
   decisions says, and the flip of that day gives the approvals, under the owner's
   standing permission for the imaging specs ("You have my permission to move them to
   ready when needed", 2026-11-19, extended to the split pieces on 2026-11-20: "you can
   flip things to ready as you get them ready").
5. **A probe build of every story.** Met on 2026-11-20 by probe round 2, for the trio as
   the split of 2026-11-20 left it. One agent built US9, US10 and US11 on 041's round-2
   probe build (036, 040, 037 and 041's US4 to US8, with 041's stand-in for 041-R17),
   tests first and with both gates, from the trio alone: history.md was not read. Every
   story built and passed both gates and wrote only its declared files, and every
   earlier test file passed unedited, 037's and 041's among them; workspace nextest ran
   1299 tests at the base and 1310 after US11, and `npm test` 545 at the base and 558
   after each story. The round found 1 blocking and 9 clarity defects, 042-R1 to
   042-R10, all folded on 2026-11-20, the anchors by the re-anchoring on `main` at `61b54a47`
   (history.md § Probe round 2 and its fold). Probe round 1, on 2026-11-19, had built
   the trio as it then stood; what this point said of it is in the same section. What
   round 2 ran:
   - each story's diff against its estimate in plan.md § Risks: US9 39,648 B (60.5 %)
     plain and 38,951 B (59.4 %) without `crates/finder/web/dist`, against 37.7 KB,
     3.3 % over; US10 22,881 B (34.9 %) as written against 23.9 KB, 4.3 % under, and
     23,087 B with the round's stand-in for R1; US11 21,834 B (33.3 %) against 20.3 KB,
     7.6 % over. At 1.17, the largest factor of the factory builds of 036, 039 and 040,
     with the cases the fold added as tests, the largest estimate is US9's 49,457 B,
     under owner choice 14's 60 KB, so no story is cut (plan.md § Risks);
   - each bound, in paused time or under fake timers, identical on all three runs:
     - US9-S9, the second tap within 5 s: armed to 4.999 s after a tap, and
       `Stop imaging run` again at 5.000 s; windows of 4 s and of 6 s both failed it,
       so it is exact to 1 ms either side;
     - US9-S2: 125.4 s and 125.6 s both read "Elapsed 2:05", 0.4 s below the next
       second;
     - US9-S12: the pause's "14:34 left" is 874.6 s rounded down, 0.4 s below "14:35";
       the time stacked is 65.4 s of 7200 s, 0.6 s below "1:06";
     - US9-S11: 14.630 GB and 3.658 GB, 0.020 GB and 0.008 GB from the edges of their
       rounding;
     - US10-S4: the three starts at one virtual instant, the third answering the rate
       limit, 1.0 s inside its second;
     - US10-S3: its two starts 1.0 s apart; a start its arguments refuse still takes
       the rate limit, so the 1 s is needed (R6);
     - US10-S6: a start after `stop` refused at 0.999 s by the rate limit, `move_axis`
       and the refused start having taken that second's two calls, and accepted from
       1.000 s, 2.0 s inside its 3 s; the direct move's reservation is gone when `stop`
       answers; no `StopAll` in the 10 s after `stop_imaging_run`, 9 s beyond the 1 s
       asked;
     - US10-S8: the read is past the pause from 0 s, 1.0 s inside the settle, and the
       tool's own answer is still `focus_pause`;
     - US10-S5: the read shows 4 stacked from 0.25 s, the first poll, 0.75 s inside the
       settle; the stop's answer reads 4 either way;
     - the slowest Rust test of the three stories, US10-S6, took about 0.21 % of
       nextest's 300 s;
   - the seams, and their state:
     - every name the trio takes from 037 and 041 matched the probe's code, and matches
       their folded trios (037's at bcf0fdfd, 041's at 534691eb). `RECORD_EVERY` was
       `Runner::RECORD_EVERY` there, an associated constant, and landed as a constant of
       the module (`crates/finder/src/imaging/run.rs:1494`); a run stopped while Track is off
       ends `Stopped: the mount was stopped.`, now in the README's row of `stopped`
       (R3);
     - 040 → 042: in 040's trio and in the probe's build, 040's US11 last writes
       `tools.rs`, `tests/mcp/main.rs` and the README's tool list, and its
       `set_steering` line follows `get_stack`'s. Folded into plan.md § The README and
       the Refinement Record (R2); the re-anchoring read each last writer again on
       `main`: 040's US11 last wrote `tools.rs` and `tests/mcp/main.rs`, and 043's US1
       wrote the README after it;
     - 041 and 035 over 031 → 042: `stop_imaging_run` and the page's Stop hang on a lost
       link without a fix of 041-R17. Settled by 043 (§ Prerequisites 2, R3). No
       scenario of this spec runs a lost link, so its tests pass either way;
     - 031's page: the block goes right after the header; under `no_link` with no run
       the page still shows only its one status, 031's not-ready and ready tests pass
       unchanged, and 031's night check (`e2e/imaging.spec.ts`) stays green with the
       block's start form on the page;
     - 016's `start_motion` refuses an MCP `start_imaging_run` during a direct move
       with `a direct axis move is running`, and 016's rate limit counts a start that
       its arguments refuse.

   Not run: a night pixel check of a run that is on (owner decision 17), a lost link
   through the page's own Stop in a browser, `start_imaging_run` from a real MCP client,
   and the field checklist. R1 stopped US10-S2 passing as written; the round's
   stand-in, its two keys, is the fold's text. The fold's new cases are not yet
   committed tests: each ran as a throwaway that passed on the probe's code four times
   and failed under the mutant it was written for, in the story it is now in.
6. **The plan document, ADR-0002, the glossary's imaging terms and the research note
   are on `main`.** Met by 2026-11-19: all four are on `main`.
7. **`main` is still as read.** Met on 2026-11-20 for what the re-anchoring read: every
   `path:line` in the trio was read again by its content on `main` at `61b54a47`. Of the
   31 the draft held, read at 49bb210, 17 were where they had been and 14 had moved and
   were fixed: the 14 that probe round 2 found stale, the README's line 57 that the
   validator refused among them. None was lost. 31 more name what 037, 040 and 041
   landed, in the code, the README and `DESIGN.md`; § Prerequisites 2 cites 043's. The
   draft's citations were first read at 49bb210 on 2026-11-15, and again there after the
   audit, when `main` had not moved; no cited line had moved then, and
   `crates/finder/src/targets.rs:39` replaced a citation one line above the `struct` it
   names. They are re-read at the flip if `main` has moved.
8. **The real mount.** Not met, and outside the factory: a GoTo to a target and Track
   have not run on the real mount (the plan document, § Before any flip, 4). The spec
   can be built before that; the run is not used at the telescope until it is. The
   field checklist's step 3 leaves Track alone on the target for 30 minutes before the
   first run: a night at the telescope logged a garbled reply from the mount every
   few minutes, and if one of those ends Track, no run outlasts it.
9. **No operator-staged files are needed.** Met.

## Owner decisions

The owner decisions of this spec's stories, 13, 17 and 19, keep the numbers they had in
the twelve-story trio. Each is a design or taste call the plan document leaves open, but
for 13, whose recommended answer the plan document now writes into its shared
interfaces. All three are settled. None was a row of the owner's decisions document,
whose recommended answers the owner approved on 2026-11-17, so each was left here for the
flip; the flip of 2026-11-20 takes each one's recommended answer under the owner's
standing permission for the imaging specs, as 037's and 041's flips took theirs. The
recommended answer of each is the answer in force.

13. **Where the run block sits on the Imaging page.** Recommended: first, above the
    Camera section, because Start is the page's main action. Instead: last.
17. **No new Playwright check.** Recommended: none. Vitest drives the run block through
    031's `useImaging`, and 041's US8's `cli` test starts the built binary. The block holds no
    picture and uses only existing classes. Instead: an `imaging-run.spec.ts` in the
    simulate project, which means an approved edit of `playwright.config.ts`.
19. **Which run tools are logged and rate-limited.** Recommended: `start_imaging_run`
    only, the one that moves the telescope. Instead: all three that change something.

## Owner approvals

Flipping this spec to `ready` approves the settled answer to every owner decision above,
and the following. Each is named in the FR or success criterion that makes it.

3. **Four MCP tools**: `start_imaging_run`, `stop_imaging_run`, `continue_imaging_run`
   and `get_imaging_run`. The first is a moving tool, and `start_motion` learns its name
   beside `set_tracking` (FR-037, FR-038).
6. **The Imaging page's run block** and its words, its `DESIGN.md` entry and its
   `.impeccable/design.json` component (FR-035, FR-036).
7. **`crates/finder/README.md`**: three rows of the API table, four tools in
   `## MCP clients`, and a new section `## An imaging run` with a field checklist
   (FR-039 to FR-041).
8. **New test modules are declared in existing test files** (SC-002). Each is one `mod`
   line, in alphabetical order, in its binary's `main.rs`: `imaging_run_docs` in `crates/finder/tests/console/main.rs` and `imaging_run` in
   `crates/finder/tests/mcp/main.rs`. No other line of
   those files and no other existing test changes.
9. **Nothing else new.** No dependency, no `Cargo.toml` change, no scope-link message,
   no config key, no console flag, no glossary term and no `unsafe` (SC-003). The
   glossary's **Imaging run** is the plan document's, and **Set point** is 036's.

## Field checks

At the telescope, by the checklist of FR-041: the whole run on a bright target,
started once from the page and once by an MCP client, and a design review of the run
block on a phone in the dark. 041's flip.md § Field checks has the checks of the run's
own parts. GoTo and Track must be proven on the real mount before any of it
(§ Prerequisites 8).
