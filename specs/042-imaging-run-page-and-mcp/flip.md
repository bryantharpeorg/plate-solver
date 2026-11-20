Not given to builders or the judge. Nothing here is a rule; the trio is.

# Before 042-imaging-run-page-and-mcp is flipped to `ready`

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed.** Met for `014-goto`, `015-tracking`, `016-mcp`
   and `017-console-stop-confirm`, and, by 2026-11-19, for `030-imaging-camera-link`,
   `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
   `033-focus-reading-and-aid`, `034-dark-and-flat-frames` and
   `038-track-follows-a-push`, whose `push` the `pushed_away` ending reads. Not met for
   `035-live-stack`, which is flipped and being built, and `036-tracking-from-subs`, a
   draft beside this one.
   Not met for `040-stack-waits-for-steady`, cut from 036 on 2026-11-20,
   nor for `037-imaging-run` and `041-imaging-run-on-the-rig`.
2. **This trio is re-anchored after `037-imaging-run` and `041-imaging-run-on-the-rig`
   land.** Not met. Every name in plan.md § What 037-imaging-run provides and § What
   041-imaging-run-on-the-rig provides is re-read on `main`, and the last story of each
   spec that writes a shared file replaces the one named in plan.md § Refinement Record. Where
   037 or 041 lands offering something else than those sections say, the story that
   reads it changes, and a gap goes to the owner before the flip.
3. **The lease of the imaging camera exists.** Nothing of this spec touches it
   (041's flip.md § Prerequisites 3).
4. **The owner decisions are settled and the approvals given** (§ Owner decisions
   still open, § Owner approvals). Not met: none of this spec's is settled as a whole.
5. **A probe build of every story.** Met on 2026-11-19 by probe round 1, for the trio
   as it then stood, as 037's flip.md § Prerequisites 5 tells. What the round settled for
   this spec's stories:
   - each story's diff against 64 KiB (spec SC-007): US9 25,710 B; US10 20,747; US11
     16,425. All were under. The fold of 2026-11-19 added code and cases to every story,
     so plan.md § Risks carries the figures as they are estimated now;
   - the earlier page tests still find their controls: all 380 earlier Vitest tests
     passed unchanged with the block on the page, and so did the Playwright suite,
     031's night check of the Imaging page among it;
   - one more defect blocked a story as written: US10's continue read the tool's own
     answer, which is still `focus_pause` (D30: it is now read back with
     `get_imaging_run` after a settle, US10-S8).

   Not settled, and for the second probe round the owner asked for (037's history.md
   § Clarifications, the owner's answer 17): everything the fold of 2026-11-19 added,
   none of which has been run, and each story's diff on the folded trio.
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

The owner decisions of this spec's stories, 13, 17 and 19, keep the numbers they had in
the twelve-story trio. Each is a design or taste call the plan document leaves open, but
for 13, whose recommended answer the plan document now writes into its shared
interfaces; they stay here, under their numbers, for the owner to settle at the flip.
None is settled as a whole. A different answer is written into the trio and re-validated
before the flip, never left to a story.

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
