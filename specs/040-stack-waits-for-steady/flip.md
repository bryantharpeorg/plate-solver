Not given to builders or the judge. Nothing here is a rule; the trio is.

# Flip: The Stack Waits for a Steady Tracker, and Shows the Steering

`040-stack-waits-for-steady`, the second of two pieces of tracking steered by the subs.
What the owner settles and approves when this spec is flipped to `ready`, what must be
met first, and what is checked outside the factory. The trio cites the owner decisions
and approvals below by number and states the rule each one settles. A bare § cite in
this file is to plan.md.

## Owner decisions

Each is a design or taste call the plan document leaves open. None is settled. A
different answer is written into the trio and re-validated before the flip, never left
to a story.

9. **Subs taken across a time the tracker was not steady.** Recommended: dropped before
   they are solved, with the reason `not_steady`, including a sub during which tracking
   ended, or was stopped and started again (reworded on 2026-11-19, AA13: a sub that
   begins with no tracker is 035's to judge, as FR-022 has it). Instead: kept, and left
   to 035's own gates.
10. **The readout's window and the drift's definition.** Recommended: the last 10
    tracking errors; a sub's drift is the change in tracking error since the sub before,
    less the steps made between them, scaled by the exposure over the time between their
    mid-exposures. Instead: the last 60 s; or the bare change between subs.
11. **MCP** (a departure). The plan document says "Each step also has its own route and
    MCP tool", and its list of tools has none for 036, whose `steering` part it has read
    through `get_imaging_camera` and the session tools. Recommended: no new tool.
    Tracking from the subs is not a step an MCP client starts: it runs inside 035's
    stack, which has its own route and its `set_stacking` tool, and with the defaults
    (the subs steer, 6 px) it needs no call at all. `get_imaging_camera` and the
    session tools carry the `steering` part, read-only, so a client sees what steers,
    the tracking error and the drift. The settings are tuning, changed from the page.
    Instead: a `set_steering` tool taking `on` and `smear_px`, behind the same
    guards as the page's route, so that the rule holds to the letter; it is one more
    story after 036's US5. Changed by the owner on 2026-11-19 (the reanalysis, choice 12):
    that tool is taken, since 037 gives an agent the whole night and an agent could
    neither run the check with and without the feedback nor change the smear before a
    restart. It takes what the route takes, `on`, `smear_px`, `step_share` and
    `rate_share`, and is User Story 11, built last (FR-031). The README says what each
    key of `steering` means and why subs are dropped (FR-025). The description
    `get_imaging_camera` gives itself in the tool listing is 031's and a test of 031's
    pins its words, so it is not changed: `set_steering`'s own description says where
    the steering is read (FF19).
12. **No Playwright test of the block.** Recommended: none, as 032 decided for its own
    block; 031's night check of the Imaging page still runs over it. Instead: a new
    check in the simulate project, which means editing `playwright.config.ts`.
17. **When the tracker is not steady.** The plan document's rule 1 gives two times as
    today's examples: "today false during a backlash take-up and the keyhole pass".
    The rule is not edited. Recommended: this decision declares three further times
    the tracker answers not steady. From a tracker's start until its first rates are
    sent, and after tracking ends, because nothing is held then. And for the whole of
    a first reversal's backlash measurement, which lasts minutes: 015 closes that
    measurement from the finder camera's pointings and its own corrections, the tube
    does not follow the drive while the gear's play is crossed (about five minutes for
    1′ of play near transit, `docs/research/2026-11-15-imaging-camera.md` § 7. Field
    rotation, sub length and the drive), and subs taken then are smeared. It happens
    once for a mount's axis; the backlash is kept afterwards. While the tracker is not
    steady its own corrections run as 015 has them, even after the subs have lapsed
    (owner decision 6), since its gaps are turned away then. Instead: steady during the
    measurement, with the subs steering through it, which changes how 015's measurement
    closes and would need 015's backlash tests proven again under it. Changed by the
    owner on 2026-11-19 (the reanalysis, choices 1, 6 and 7): three more times. First,
    while the session reads a push by hand as moving (038), from the live pointing that
    makes it so until the push is followed or the telescope is back within 038's
    threshold: the tube is not where the tracker holds it, and as drafted exposures went
    on with a hand on the tube and a sub taken before 038 followed the push steered the
    telescope back toward the old place at 30″ a gap (AA2, BB3, CC2, DD3, FF1); the cost
    is that one finder solve more than 5′ wrong drops one sub. Second, for 2 s after the
    rates that follow a tracker's start, a take-up and the keyhole pass, because a real
    tube rings after each (AA10, FF8). Third, for the turn in which the tracker finds an
    axis at rest whose last rate was not zero: the scope node stops both axes on a
    failed rate command and still acknowledges it (FF3).

Decisions 1 to 8 and 13 to 16 are approved by 036's flip (036's flip.md § Owner
decisions); this trio cites one of them, 036's owner decision 2, by that name.

## Owner approvals

Flipping this spec to `ready` approves the recommended answer to each owner decision
above, and the following, under the numbers 036's flip.md gives the rest. Each is named
in the FR or success criterion that makes it.

3. **The MCP tool `set_steering`** (FR-031). 036's approval 3 gives the route, the
   `steering` part and the module. The plan document's § Interfaces, in this worktree,
   names no MCP tool for 036; owner decision 11's answer of 2026-11-19 corrects its list
   of tools in the same change as the flip (flip.md § Prerequisites 9).
6. **035's stack changes in four ways** (FR-023): `Dropped` gains a twelfth count,
   `not_steady`, written last and always; a sub may end with the outcome `not_steady`;
   the taker waits for the steering before every exposure; and before a stack's first
   sub the taker may set the console's `exposure_ms` to a shorter exposure than the
   observer set, through the stack's own lease, the taker putting the observer's back
   through the same lease whenever it returns, before the lease drops with it. A stack
   whose first change of the exposure is refused ends with the reason `camera`. No
   existing test changes: 035's tests
   read `dropped` key by key and never assert how many keys it holds (035 spec.md
   § User Scenarios), and flip.md § Prerequisites 3 checks that in 035's landed tests
   before the flip.
7. **The Imaging page's block `Tracking from the subs`**, its words, its `DESIGN.md`
   entry and its component in `.impeccable/design.json`; and the words
   `while the mount was not steady` in 035's `Dropped` line (FR-027, FR-028, FR-029).
8. **A README section and the lines of one MCP tool** in `crates/finder/README.md`
   (FR-025, FR-031); 036's approval 8 gives the route's row.
9. **Each new test module is declared in an existing test file** (SC-002): one `mod`
   line a module, in alphabetical order, in `crates/finder/tests/console/main.rs`
   (`steering_readout`, `steering_steady`), `crates/finder/tests/motion/main.rs`
   (`steering_stack`, `steering_steady`, `steering_unsteady`) and
   `crates/finder/tests/mcp/main.rs` (`steering`), as 027's layout asks of every finder
   test. `steering_unsteady` is the module of User Story 10, cut from User Story 6, and
   `steering` of `mcp` is User Story 11's: the approval's own rule, one `mod` line for
   each new finder test module, applied to two more. No other line of those files and no
   other existing test changes.

## Prerequisites before this spec is flipped to `ready`

1. **036 has landed**, and with it what 036 depends on: `015-tracking`,
   `030-imaging-camera-link`, `031-imaging-frames-on-console`,
   `032-imaging-solve-and-aim-point`, `033-focus-reading-and-aid`,
   `034-dark-and-flat-frames`, `035-live-stack` and `038-track-follows-a-push`, all in
   this spec's `depends_on_landed` with `036-tracking-from-subs`. Met: 036 was flipped
   to `ready` (`6287c51b`, #325) and landed on 2026-11-20 with its seven stories, US2 at
   `c24f88cf` (#327), US1 at `8b13dcaf` (#328), US3 at `89ac00cf` (#329), US9 at
   `950355a0` (#330), US4 at `1ad49378` (#331), US5 at `72877199` (#332) and US12 at
   `fc161a54` (#333). What 036 depends on had landed before it, each with its last
   story's merge (036's flip.md § Prerequisites 1): 015 at `a02dc35b` (US9, #177), 030
   at `49f8dcd2` (US8, #269), 031 at `e9687ce9` (US9, #279), 032 at `670ddcf9` (US7,
   #288), 033 at `80f414ff` (US7, #298), 034 at `002dbef8` (US8, #309), 035 at
   `52785403` (US10, #321) and 038 at `1c2b427b` (US3, #260).
2. **The trio is re-anchored against `main` after 036 lands.** Met on 2026-11-20, on
   `main` at `fc161a54`. Each point of § What 036 provides was read in 036's landed code
   and corrected where it differed: the cap's turn is the private `turn_for`, letting the
   tracker go is `let_go`, and `SteeringEdit` refuses anything but a JSON object by a
   `Deserialize` of its own; the section now cites that code by line, and no difference
   found changes what a story builds. Every `path:line` here, read at 49bb210, was read
   again by its content and its number put right: 038 and 036 had moved the citations of
   `crates/finder/src/tracking.rs` (`impl Drop for Tracker`, `keyhole`, `take_up`, and
   where a measurement opens and closes), 036's US1 those of the simulator's
   `controls.rs`, and 038 those of `model.rs` and of the `dl` of `TrackControl.tsx`. The
   names 036's re-anchoring left to this one (`set`, `FrameView`, `Refusal`,
   `putImagingSettings`, `set_unresponsive`, the camera's lease, `set_solver`,
   `solve_frame` and `SimulatedSolver`) are as the trio names them. The cites of 036's
   US5 follow 036's cut of US5 into US12 (036's history.md). The story that last wrote
   each shared file was read with `git log` on `main`: `crates/finder/tests/motion/main.rs`
   is now 036's US12's, and the rest matched. The validator was re-run there.
3. **035 is read as landed, against § What 035 provides.** Met on 2026-11-20, on `main`
   at `fc161a54`, where 035's `stack/service.rs` and `StackBlock.tsx` are as 035 left
   them (`6a519d49`, `6e506ec6`): each of its nine points was read in the landed code
   and holds as written, and § What 035 provides now gives each its lines. These were
   settled:
   - the taker's loop, its returns, and the worker's `process` and `run` are where § The
     stack puts US7's calls, by name and line (read by name on `main` on 2026-11-19,
     after 035's US8 landed, and by line at `fc161a54`: § What 035 provides gives the
     lines);
   - how the worker counts and lists a dropped sub, and what `Dropped::count` does with
     an outcome it does not name: on `main` it is `unreachable!`
     (`crates/finder/src/imaging/stack/service.rs:123`), and US7 gives
     `not_steady` its arm;
   - the stopping mark is the `tokio::sync::watch` of `bool` that 035's plan.md names
     (point 2), and the taker holds a receiver of it (both hold, `service.rs:488` and
     `:769`);
   - the taker owns the run's `Lease` and it drops as the taker's task ends, and the
     holder's `set` behaves as point 3 says (both hold, `service.rs:508` and
     `crates/finder/src/imaging/mod.rs:901`). This replaces the first draft's use of
     034's `Calibration::restore_exposure`, which called `Imaging::set`, would be refused
     while the stack holds the camera and has since been removed from 034 (034 A1), and
     the second draft's restore in 035's ending, after the taker had returned and the
     lease with it;
   - 035's landed `crates/finder/src/imaging/stack/service.rs` holds none of the
     thirteen names 036's US2-S6 lists, so that US7-S6 rests on US7's own diff (read on `main`
     at `52785403` on 2026-11-19, and again at `fc161a54` on 2026-11-20: none). If it
     holds one, the trio goes back to the owner before any story is dispatched;
   - no landed test compares an imaging view, or the session's `imaging` object, whole
     on a console that tracks with its camera ready: from US7 such a view carries
     `steering` (§ Shapes that existing tests pin). None does at `fc161a54`, 036's seven
     test files among them. 036's US5-S5 compares the whole `steering` part before and
     after refused edits (`crates/finder/tests/console/steering_cap.rs:602`), on 032's
     solving rig, which does not track: two reads of one steering, which the keys US6
     and US8 add leave equal there;
   - 031's MCP test still asserts `get_imaging_camera`'s description word for word, so
     US11 leaves it, and the test that calls every listed tool with `{}` still does so
     on a console with no imaging (both hold, `crates/finder/tests/mcp/imaging.rs:199`
     and `crates/finder/tests/mcp/motion.rs:528`; no `mcp` test changed after 035);
   - no test of 035 counts `dropped`'s keys, compares the whole object with a literal, a
     fixed list of keys, a golden file or a schema that lists its keys (a comparison of
     two serializations of the same `Dropped`, as
     `stack_console.rs` makes between `stack.json` and the view, is not one), or builds
     `Dropped { .. }` by a struct literal without `..Default::default()` (none does at
     `fc161a54`: `Dropped {` is found only in the type and its `impl`, and
     `stack_console.rs` and `stack_loop.rs` read whatever keys `dropped` holds). If one
     does, the trio goes back to the owner for an approval that names that test, before
     any story is dispatched: no approval here covers an unnamed test;
   - 035's `StackBlock` builds its `Dropped` line as § The page assumes (it does,
     `crates/finder/web/src/components/StackBlock.tsx:62`);
   - no landed test of 031 to 035 reads a region of the Imaging page by its place from
     the page's end (`getAllByRole("region").at(-1)` or `.at(-2)`): this spec's block
     becomes the page's last region. 034's US9-S1 places its block after `Imaging frame`
     and `Focus`, and 035's US9-S6 its region after Calibration (none does at `fc161a54`:
     the two `.at(-1)` there read arrays, not regions);
4. **The owner decisions are settled and the approvals given** (§ Owner decisions and
   § Owner approvals above). Not met: this spec's decisions, 9 to 12 and 17, are open,
   and 036's flip settles the rest.
5. **A probe build.** Met on 2026-11-20 by probe round 2. Probe round 1 built US6, US7
   and US8 on 2026-11-18 and 19 as stories of 036 (036's flip.md § Prerequisites 5 has
   what it settled, and 036's history.md how). Round 2 built this spec's five stories
   from the trio, in two agents (US6 and US10, then US7, US8 and US11), one commit a
   story in the Work Graph's order, tests first and with both gates green, each story
   writing only its declared files; one PROBE-ONLY commit outside every story built the
   fix of a plan rule (040-R2, now folded). It found 3 blocking and 20 clarity defects, 040-R1 to
   040-R23 (history.md, probe round 2 and its fold). It built on 036's probe code, the
   tip of 036's round 2 split (`a0602839`), not on `main`: the re-anchoring compared that
   code with 036's landed code (`a0602839..fc161a54`), and found every point of § What
   036 provides the same and no difference that changes what a 040 story builds; the
   trio names the landed code throughout. It showed:
   - **The bounds**, each on three runs with the same digits:
     - US7-S1 from the 14th sub: at most 0.014181″, against 0.15″ (10.6×; round 1
       0.01558″, 9.6×); the 12th sub's drift 0.030450″, against the model's 0.030″.
     - US6-S5, with eight gaps and the offset left alone after the lapse: the largest
       true error 41.412530″, against 015's 60″ (1.449×), while the first reversal's
       measurement is open, as round 1's 41.414″ with three gaps and the integral held;
       at most 10.078066″ after the close (round 1 10.850″); the play measured
       0.299879°, 0.43″ off the 0.3° configured, against 30″.
     - US6-S1: the gap asked during the take-up answered 2.150 s before L, against at
       least 1.5 s.
     - US7-S2's limit equal to the test's own figure within 2 × 10⁻¹⁶, against 2%;
       US7-S3 one sub dropped `not_steady`, against at least one; US7-S7 `far_arcsec`
       471.98″, within 420″ to 540″, and the eyepiece within 2.045″ of its first
       reading, against 60″; US7-S8 12 of 12 subs over 0.5″ (0.7452″ to 0.7466″, the
       model's 0.749″), against at least 8; and US7-S9's stop answered at once, against
       0.5 s.
     - US10-S1, US6-S8 and US10-S9: the feed-forward plus the offset in force at the
       gap's answer to 0.000000″/s, against 0.01″/s; and US10-S9's last azimuth rate
       before the gap 7.4189″/s off it, against more than 1″/s.
     - US10-S8, which had no bound: as written it failed (040-R3, the owner's).
   - **The cases nobody had run** all ran and passed as written:
     - the stopped-axis test raising no false alarm (§ Plan traps 32): with its branch
       printing, the 165 motion tests at US6's tip and the 174 at US10's, 036's US3's,
       US9's and US4's modules among them, raised one stall, US6-S7's own;
     - US6-S7: the stall found at the turn at r + 1 s, with `sent`'s time r;
     - US6-S6's push, on the goto rig;
     - US7-S7's far stack, and US7-S9's and US7-S10's gaps that a stop and a lost link
       end;
     - US10-S9's worked 5.4 s, confirmed: the interval was 3.410 s;
     - US6-S17's wait for the limit, on the scripted tracker;
     - US10-S1, US6-S8 and US6-S9, what the mount is left running after a time not
       steady;
     - the settle's times: 2.150 s before L in US6-S1, and steady exactly 2.000 s after
       the first rates in US10-S3 (040-R4 and 040-R10 folded what they showed);
     - US8-S1's worked figures, all six to 10⁻⁴.
   - **Every test file passed unedited under every later story:** 036's under US6, the
     sha256 of every existing test file the same before and after it and 036's motion
     and console modules passing on three runs; US6's under US10; every earlier one
     under US7, US8 and US11, with 035's three page tests among the 545 and 031's MCP
     tests. US10's on US6's code, with none failing first: not as written. 7 of 9
     passed at the first run; US10-S7 failed on the plan's rule for a take-up's word
     (040-R2, folded: the keyhole pass's own take-up keeps `overhead`), and US10-S8 as
     written (040-R3); with that fix and US10-S8 read from the player's last false
     reading of `steady()`, 9 of 9 on three runs. Every other story's tests failed first, but
     US7-S6, the guard, which passes on any code without the thirteen names, as
     § Prerequisites 3 means it to.
   - **Each story's diff against its estimate** (§ Risks; 65,536 B the budget): US6
     66,145 B (100.9%) against 48,400 B, 36.7% over and over the budget (040-R1, the
     owner's); US10 16,978 B (25.9%) against 15,300 B, 11.0% over; US7 43,790 B (66.8%)
     against 43,500 B, 0.7% over; US8 47,784 B (72.9%, `dist/` counted as binary)
     against 43,700 B, 9.3% over; and US11 12,354 B (18.9%) against 8,700 B, 42.0%
     over, most of it the imaging console its own module builds.
   - **Not run:** the msrv gate on Rust 1.96, coverage and CI; no real hardware; and
     040-R20's schema, folded as its report wrote it, was not built.
6. **Outside the factory, before this spec is used at the telescope:** GoTo and Track
   proven on the real mount (the plan document, § Before any flip, 4), and its backlash
   measured on both axes: altitude reverses at transit, and azimuth too for a target
   that culminates north of the zenith. Not met.
7. **The documents this trio cites are on `main`.** Met: the plan document
   (`docs/plans/2026-11-15-specs-030-037-imaging.md`), ADR-0002 and the research note
   (`docs/research/2026-11-15-imaging-camera.md`) have been on `main` since 030's flip
   (`55e41f57`), and are there at `fc161a54`.
8. **No operator-staged files are needed.** Met.
9. **The plan document agrees with the settled decisions.** Not met: on `main` its list
   of MCP tools has none for 036
   (`docs/plans/2026-11-15-specs-030-037-imaging.md:216`); owner decision 11's answer of
   2026-11-19 gives this work one, `set_steering`, and the plan document's list gains it
   in the same change as the flip.
   If the owner settles decision 11 otherwise, the plan document is corrected in the
   same change. Rule 1 is not edited: owner decision 17 declares the further times a
   tracker is not steady on its own.
10. **031 carries the camera's lease.** Met: 031's US9 landed it on `main`
    (`e9687ce9`), with `Imaging::lease`, `Lease::take` and `Lease::set` as this trio
    names them (`crates/finder/src/imaging/mod.rs:736`,
    `crates/finder/src/imaging/mod.rs:909`, `crates/finder/src/imaging/mod.rs:901` at
    `fc161a54`), and 035 holds it for a stack. This trio reaches the lease only through
    § What 035 provides, points 3 to 5.

## Field checks, outside the factory

Before this spec is used at the telescope (flip.md § Prerequisites 6):

- a run through the meridian;
- Track left on for 30 minutes with the scope node's journal read afterwards for
  `mount command failed`, each of which is a stop of both axes that the console is not
  told of;
- ten minutes of Track with the axis counts read once a second, to see whether the axes
  advance at the rate sent or fall behind by a stop at every rate change;
- how long the first reversal's measurement takes to close with the real finder camera's
  scatter;
- the Impeccable review of the block, after US8 lands.

The checks of 036's stories are in 036's flip.md.
