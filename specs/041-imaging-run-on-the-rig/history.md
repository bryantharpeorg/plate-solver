Not given to builders or the judge. Nothing here is a rule; the trio is.

# How 041-imaging-run-on-the-rig got here

## Probe round 2 and its fold (2026-11-20)

Probe round 2 built this piece's five stories, as the split of 2026-11-20 left them, on
2026-11-20: one agent built US4 and US5, a second US6, US7 and US8, starting where the
first stopped, on a probe base of 036's, 040's and 037's round-2 probe builds, tests
first, with both gates and from the trio alone (flip.md § Prerequisites 5 says what it
ran). Every story built, passed both gates and wrote only its declared files. The round
found 1 blocking and 30 clarity defects, 041-R1 to 041-R31. They were folded on
2026-11-20, while 040 was building, after 037's own round-2 fold and before the
re-anchoring onto `main`, which is a step of its own and comes after 037 has landed.
Where a scenario gained a case, it has the words of the throwaway test the probe ran,
which passed on the probe's code and failed under the mutation it was written for, in
the story the probe named.

| Defect | What it found | Where it went |
|---|---|---|
| 041-R1 | US4-S1's window of 9.5 s did not pin 032's 10 s reach: the first frame came 12.1 s after `tracking`, and a reach of 8 s survived (A15) | US4-S1's bullet, T016 and the record: 11.5 s (the owner's answer, below), with the throwaway's words |
| 041-R2 | Nothing pinned `PUSH_GRACE` from above: a grace of 60 s survived (A6) | a US4-S4 case, a push that keeps moving ends the run 10 s to 15 s after it; T017; the record |
| 041-R3 | No scenario changed the target between `target` and `point` (A18) | a US4-S5 case, `point` refuses with `the target was changed`; T017; the record |
| 041-R4 | The `track.target.id` clause of `state`'s second row was unpinned (A2) | a US4-S5 case, Track turned to another target reads `target_changed`; T017; the record |
| 041-R5 | The generation row of `state` was unpinned once Track is turned on again (A3) | a US4-S5 case, Track turned on after a Stop still reads `stopped`; T017; the record |
| 041-R6 | No rig scenario had Track refuse a re-centre (R5) | a US4-S5 case, `repoint` fails as the mount in Track's words; T017; the record |
| 041-R7 | `near_arcsec` from the wrong share survived (V3) | a US4-S5 case, `near_arcsec` 449″ within 1″; T017; the record |
| 041-R8 | US5-S3's count started again passed without it (M5) | that bullet: offset 50, eight solves and no warning; T020; the record |
| 041-R9 | The second `centring` message after a try was unpinned (M18) | a US5-S3 case; T020; the record |
| 041-R10 | No scenario sat on either inclusive bound (M20, M7) | a US5-S3 case at exactly 90″ and at 450″; T020, which says the 90″ case passes before T022 |
| 041-R11 | `FRESH_POLLS` of 1 and of 200 both survived (U5, U6) | a US5-S4 case with a finder frame every 2 s, and US5-S8, the mount part alone with no live pointing; the edge case; T021, T024; the record |
| 041-R12 | "US5-S5 catches two removed together" was not exact: only step 3's check with step 5 (P3) | plan.md § The console's mount part, after `repoint`'s steps, and § Risks |
| 041-R13 | The first live pointing after the re-centre's Track off came within two polls, 0.50 s, not 0.30 s | US5-S4, plan.md § Risks and flip.md § Prerequisites 5 |
| 041-R14 | US4 and US5 over their estimates; at 1.35, US4 with the round's cases near 64 KiB | plan.md § Risks at 1.17, under owner choice 14: no cut (below) |
| 041-R15 | plan.md cites lines that 036, 040 and 037 moved | not folded: the re-anchoring's (below) |
| 041-R16 | A target set between step 3a and Track's own read of it starts Track on the new target | plan.md § Risks, the row of a target changed after a re-centre |
| 041-R17 | Blocking. 035's `Stacker::stop` awaits 031's `Imaging::abort`, which a scope node that has stopped answering never answers, so US8-S6's run never finished its ending; the camera part's abort (US4) awaits the same call, not verified | settled by the owner with 043, which covers the camera part's abort as well (below): flip.md § Prerequisites 2 and plan.md § What 030 to 036, 038, 040 and 043 offer; the stand-in not folded |
| 041-R18 | US7's reads fell on the run's 250 ms poll, so limits 0.6 s short survived (L7, L26) | US7-S2, US7-S3, US7-S6 and US7-S8 read 0.6 s off a whole second; T030; the record |
| 041-R19 | T030 and T033 called three tests failing that pass before the code: US7-S5, US7-S9 and US8-S9 | T030 and T033 |
| 041-R20 | § Risks said no `save_error` is reached on the rig; a read-only stack folder reaches it (S11, S23) | plan.md § Risks; the two cases are not added (the owner's answer, below) |
| 041-R21 | The focus part's `SolverUnavailable` row was unreached (P7) | a US6-S7 case on a rig with no imaging solver; T026; the record |
| 041-R22 | Four stated rules of the repeat and of a pause's words were unpinned (F9, F11, F17, F20) | three US6-S8 cases and one US6-S10 case; T026; the record |
| 041-R23 | `pause_ends_at` cleared at a resume was unpinned, and at an ending unstated (F21, F27) | plan.md § The focus gate in the machine, step 3; US6-S2, US6-S3 and US6-S5; T025, T027; the record |
| 041-R24 | Who answers a pause when a resume and the limit come in the same instant was not said | plan.md § The focus gate in the machine, step 2; T027 |
| 041-R25 | No test can resume while `open_aid` is awaited: 037's script has no `Step::OpenAid` | plan.md § The focus gate in the machine, step 1: the order is the rule (below) |
| 041-R26 | Whether a check that did not finish is repeated when the tracker was not steady was not said | plan.md § The focus gate in the machine and T027: it is not (below) |
| 041-R27 | Four stated rules of the limits were unpinned (L18, L20, L22, L24) | cases in US7-S3, US7-S4 and US7-S9; T030; the record |
| 041-R28 | `reserve_bytes`' rounding up and its binning of 0 were unreached on the rig (S19) | a US8-S7 case on the plan's worked figures; T033; the record |
| 041-R29 | A failure to remove `.reserve` other than a missing file was not said | plan.md § The console's stack part, `stop`; T034 |
| 041-R30 | US6 and US8 over their estimates | plan.md § Risks (below) |
| 041-R31 | Two cited lines in US6 to US8's text moved | not folded: the re-anchoring's (below) |

- Q: How was 041-R17 settled? → A: By the owner, on 2026-11-20, with a fix spec of its
  own, `043-imaging-abort-answers-in-time`: 031's `Imaging::abort` waits for the node's
  answer for at most `ABORT_WAIT`, 5 s, and a call not answered by then is dropped and
  ends the frame `aborted` (043's two decisions, "Yes, take them"). 043 lands after 040
  and before 037. With it the stack part's `stop` stays `stacker.stop().await` as the
  trio wrote it: the probe's stand-in, a race of the stop against the stack's own
  `ended` view, is not folded, and US8-S6 is unchanged. The camera part's `abort` (US4,
  `console.rs`), which awaits the same `Imaging::abort` and which the round left not
  verified on a lost link, is covered by 043 as well, and so are 042's stop route and
  tool (042-R3). flip.md § Prerequisites 2 has the point. `depends_on_landed` gains
  `043-imaging-abort-answers-in-time` at the re-anchoring, not now: 043's trio is not on
  `main`, and the validator refuses an edge to a spec it cannot find.
- Q: What did the sizes come to, under owner choice 14? → A: No cut. The owner took
  choice 14 on 2026-11-19: "if the fold's estimate passes 60 KB, cut it by tests into a
  new story". The night's factory builds ran 0.93 and 1.04 times their probes' diffs
  (039), 0.94 to 1.17 (036) and 0.85 and 0.99 (040's US6 and US10), so the fold took
  the highest, 1.17. Each estimate is the story's round-2 diff plus the diff bytes of
  the throwaways the fold makes cases of, each counted whole, with a `+` a line, though
  some only add asserts to a test the story has anyway; a case that only changes a
  number counts 0:
  - US4: (41,483 + 4,715) × 1.17 = 54,052 B, the 4,715 being R2's 823, R3's 637, R4's
    781, R5's 873, R6's 700 and R7's 901; R1 changes a number;
  - US5: (31,921 + 5,195) × 1.17 = 43,426 B: R8's 670, R9's 1,033, R10's 497 and 549,
    R11's 1,004, and its parts-alone case at 1,442, the throwaway's 734 with 408 for
    US4's `steady_within` written again in US5's module and about 300 for the aim
    point and the imports;
  - US6: (41,188 + 5,708) × 1.17 = 54,868 B: R21's 2,376, R22's 526, 774, 718 and 678,
    and R23's 636;
  - US7: (14,797 + 2,074) × 1.17 = 19,739 B: R27's 496, 613, 473 and 492; R18 changes
    numbers;
  - US8: (32,398 − 910 + 406) × 1.17 = 37,316 B: the round's diff less R17's stand-in,
    and R28's 406.

  Every story is at or under 60,000 B. US6 and US4 are the stories to watch, at 84 % and
  82 % of 64 KiB.
- Q: Which choices did the round leave to the owner? → A: Four. The owner took the seat's recommendation for each on 2026-11-20 ("Yes, take them").
  - R1: 11.5 s, which also catches a reach of 9 s, whose first frame would come at
    11.1 s, with 0.6 s of margin. The probe's case ran with 11 s; the bound is the only
    change.
  - R20: two US8 cases for a `save_error`, a stack folder made read-only while the run
    stacks, stopped once from the run and once from 035's route. They do not hold for a
    test run as root. With both, US8's estimate is (31,894 + 1,881) × 1.17 = 39,517 B.
    Not added: a builder or CI run as root would fail them.
  - R25: the fold took the plan's sentence, option (b). Option (a), a `Step::OpenAid` in
    037's script and a US6-S9 case, is an edit to 037's file, and no probe ran it.
    Option (b) stands.
  - R26: the fold says a check that did not finish is not repeated, which is what the
    trio's rule already said and what the probe built. The other answer, repeating one
    that did not finish with `NotSolved`, since stars trailed by an unsteady tracker are
    a likely cause of 033's `no_region`, is a change of rule. The rule stands: a check
    that did not finish is not repeated.
- Q: What of 037's own fold did this trio need? → A: Every name the trio takes from 037
  was read against 037's folded trio (`specs/037-ready`, bcf0fdfd). `Warning` is now a
  public enum with `words` (037-R1); nothing here names it, and the tests still type
  each warning in. `SubsFailing`'s `{w}` is 041's US7's to word (037-R7), as plan.md
  § Limits and the stack's endings in the machine already does. US12's script tests are
  US13's (037-R2); this trio cites only US12-S1 and "037's US12" for the script, both
  unchanged. The record's `preserved_behavior` of US6, US7 and US8 read "037's US1 to
  US5", "to US6" and "to US7", left from the twelve-story numbering; they now read
  037's US1 to US3 and this spec's stories.
- Q: Which anchors are the re-anchoring's? → A: R15's and R31's, left as they stand,
  and one the validator already reported on the untouched draft. The re-anchoring also
  adds `043-imaging-abort-answers-in-time` to `depends_on_landed` and to the header's
  comment (flip.md § Prerequisites 2).

  | Citation | What it names | Where the round found it |
  |---|---|---|
  | plan.md, `crates/finder/src/session.rs` line 463 | the pointing's `seq` | 566 |
  | plan.md, `crates/finder/src/session.rs` line 950 | `stop_tracking` | 1125 |
  | plan.md, `crates/finder/src/session.rs` line 854 | `start_tracking`'s `stopped` | about 992 and 1009, inside `start_tracking` at 952 |
  | plan.md, `crates/finder/src/session.rs` line 508 | the mount view's `moving` | 611 |
  | plan.md, `crates/finder/src/api.rs` line 904 | `stop` | 1081 |
  | spec.md and plan.md, `crates/finder/src/tracking.rs` line 31 | the 10″ deadband | 33, `DEADBAND_ARCSEC` |
  | plan.md, `crates/finder/src/imaging/relation.rs` line 205 | the reach | 293; `FIT_REACH_S` at 36 |
  | spec.md and plan.md, `crates/finder/src/tracking.rs` lines 219 and 248 | Stop's acknowledgement; the 5 s within which Track expects a live pointing | moved; not looked for |
  | plan.md, `crates/finder/src/main.rs` line 114 | `.with_live()` | 200 |
  | spec.md, `crates/finder/src/api.rs` line 613 | `goto_target`'s refusal `Track is on` | 782 |
  | spec.md, `crates/simulator/src/mount.rs` line 269 | `push_by` | blank at 2abc9a6b: the validator's finding on the untouched draft |

  `crates/finder/src/goto.rs` line 739, `the mount is already moving`, holds.
- Q: What did flip.md § Prerequisites 5 say of round 1? → A: This, until the fold put
  round 2's points in its place:

  > 5. **A probe build of every story.** Met on 2026-11-19 by probe round 1, for the trio
  > as it then stood, as 037's flip.md § Prerequisites 5 tells. What the round settled for
  > this spec's stories:
  > - each story's diff against 64 KiB (spec SC-007): US4 32,441 B; US5 23,300; US6
  >   31,886; US7 11,298; US8 20,250. All were under. The fold of 2026-11-19 added code
  >   and cases to every story, so plan.md § Risks carries the figures as they are
  >   estimated now;
  > - US4-S1 leaves the target 4.43″ from the centre by the truth with the true aim
  >   point set, against its 110″, and its solved offset is 0.07″ from the truth,
  >   against 30″;
  > - US5-S4 centres in one try, 8.67″ by the truth, with the aim point 0.017 and 0.009
  >   pixels from the true one, against 2.0. It did so only once the mount part read
  >   steady no sooner than 032's fit reach after Track first worked: an imaging
  >   exposure started 8.2 s after `tracking` read `moving` and took no aim reading,
  >   and one started 9.2 s after it read `measured` (D14, blocking). The round built
  >   that as a fixed wait of 10 s; the fold wrote `steady_between` in its place (the
  >   owner's answer 2), which no probe has run;
  > - the simulator's mount did not move toward the newly measured aim point between
  >   the first imaging solve and the re-centre's Track off (0.00″), the pointing
  >   model's slip was not marked then, and the first live pointing after that Track
  >   off came 0.30 s later, against the 5 s of `repoint`'s step 3;
  > - the simulated axes are at rest by the time a re-centre's Track asks GoTo, and
  >   Track itself waits up to 2 s for that; GoTo did not refuse (plan.md § Risks);
  > - US6-S4's pause ended at 900.000 s and US7-S2's limit at 60.000 s, each on a poll.
  >   Their reads were a second either side, and a limit a second short survived
  >   them, so both now read half a second before and after (D22, D25);
  > - the run rig syncs, slews and solves at dawn: US4-S4's run stacked for 31 s before
  >   `sun_up`;
  > - US8-S1 reached three stacked subs on the simulator with 036's steering in the
  >   loop, none dropped, on all three runs;
  > - one more defect blocked a story as written: US4-S5 read rest the instant after a
  >   Stop, while the axes still braked (D13: it now reads from 2 s after).
  >
  > Not settled, and for the second probe round the owner asked for (037's history.md
  > § Clarifications, the owner's answer 17): everything the fold of 2026-11-19 added,
  > none of which has been run; each story's diff on the folded trio; what 036's own fold
  > changed underneath this trio the same day (037's history.md, the Refinement Record's
  > prose, "036's second fold"): the two forms of the warnings in US5-S3, and every wait
  > for a steady tracker 2 s longer on the simulator, inside which US4-S5's 900 s and
  > 120 s must still hold; and the margin of US4-S4's "within 60 s" for `pushed_away`,
  > the one bound round 1 did not time.

## The split (2026-11-20)

This spec held US4 to US8 of `037-imaging-run` until the owner approved cutting that trio
into three on 2026-11-20. 037's history.md § The split says how and why, which story went
where and which Work Graph edges and shared writes changed. What serves the whole run is
there and not here: the drafting header, the Clarifications, settled owner decision 26
and the Refinement Record's prose. Here is what came with US4 to US8.

Below § The split, the text is as it stood before the split: its story, scenario, FR,
decision and section numbers are those of the twelve-story trio, every one of which kept
its number. A story of US4 to US8 is now in `041-imaging-run-on-the-rig`, one of US9 to
US11 in `042-imaging-run-page-and-mcp`, and an owner decision in the flip.md of the spec
that owns it; a story of 036 that went to `040-stack-waits-for-steady` is cited here as
036's.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| The run points by asking Track, and re-centres by Track off and Track on | Calling `GoTo::start` from the run; feeding the tracker an offset; a small axis move | GoTo refuses while the mount tracks (`crates/finder/src/goto.rs:739`); Track already decides whether a GoTo is needed and carries every guard. An axis move is a second way to move the mount, which the plan document forbids (rules 1 and 5). |
| A re-centre turns Track off, then waits for the finder camera's next live pointing, for at most 5 s, before it asks Track again | Asking Track at once after Track off; waiting with Track still on; a count of the finder camera's frames | While Track is on 032 holds a newly measured aim point back and applies it at the first live solve the session records with Track off, before that solve's pointing is taken (032's FR-036; spec owner decision 26). The pointing the session holds was taken at the aim point of its own moment (`crates/finder/src/session.rs:1192`): asked at once, Track's 1′ test would read one taken at the old aim point, start no GoTo, and a try would be spent. A wait made with Track still on would see no pointing at the new aim point at all. The bound is in seconds because a count of star frames has no end on a finder camera that has stopped solving, and 5 s is the interval within which 015's tracker already expects a live pointing (`crates/finder/src/tracking.rs:248`). |
| Stop is told from the run's own Track off by GoTo's stop generation | Reading `track.ended_reason` | The run's own `stop_tracking` leaves `stopped` there too. Every real Stop goes through `GoTo::stop`, which advances the generation first. |
| The mount part asks for Stop again when the generation changed across its restart, and compares the generation before it reads Track's answer | Trusting Track's own check; reading Track's error first | A Stop that lands between Track off and Track's start is seen by neither. One that lands during Track's start makes Track answer `stopped` (`crates/finder/src/session.rs:854`) or GoTo answer `the observer stopped it` (`crates/finder/src/goto.rs:626`): read first, that is a `mount_failed` after the observer pressed Stop. |
| The mount part names only `armed` and `off` of Track's states, and takes `steady` from the tracker | Listing `tracking` and `keyhole_wait` | The keyhole is one mount's way of being unsteady; a later tracker has others (rule 1). |
| Subs that are not kept are removed when the stack has ended, from `stack.json` | 035 removing each as it goes | Spec owner decision 22: 035's draft keeps every sub and lists it; removing by its list needs no change to 035. |
| The mount part reads steady only when 036's tracker was steady for the whole of 032's fit reach (`steady_between`) | Round 1's fixed wait of 10 s from the first reading of a working Track; no wait | A fixed wait starts at the wrong moment, since the tracker is not steady when Track first works, leaves out exposures longer than 10 s, is a clock inside a getter, and does not start again after a keyhole pass or a take-up. 036 already keeps the list that answers the question. The lasting fix is in 032 (§ Risks). |
| A push by hand ends the run when it has settled or lasted `PUSH_GRACE` | Ending on the first `moving` reading | 038 itself forgives a push that springs back; the first reading is one live pointing (spec owner decision 27, as changed on 2026-11-19). |
| Subs that keep failing end the run by time, 600 s of stacking with a failed sub and none stacked | A count of ten subs; a page setting for the minutes | A count is under a minute of cloud at short subs and ten minutes at long ones. The owner took the constant and no setting (spec owner decision 9, as changed on 2026-11-19). |
| The offset is the separation of the solve's centre from the target | The target's pixel in the imaging frame | It needs no pixel, so it works when the target is outside the imaging frame, and it uses the same great-circle function as Track's 1′ test. |
| A region of interest refuses a start | Measuring at the sensor's centre through the solve's map | Spec owner decision 24: 032 takes aim readings from whole-sensor imaging frames only, so such a run could not re-centre. |
| The tolerance has a floor of 90″ | A share of the imaging frame alone | GoTo arrives at 60″ (`crates/finder/src/goto.rs:1013`); on a small or long-focus imaging frame a share alone would ask for less than GoTo can give and never be met. |
| A solve that did not move the aim point is retried without a re-centre and without counting a try, four in a row ending the wait (a target near the centre is then stacked, and beyond that the run ends `not_centred`), and whether it moved is read from the session, an aim point it holds back counting as moved | Re-centre anyway; counting it as a try, as this plan did until 2026-11-19; deciding by 032's outcome word | The aim point in force did not change and none is held back, so Track off and on would bring the mount where it already is, and the restart would spoil the next aim reading. 032's outcome `measured` does not say the aim point moved: 032 leaves an aim point alone when its median is within 10″ of it (032 plan.md § The relation). |
| The camera part asks for the lease before it looks for an imaging frame in flight | The in-flight check in `ready`, before the lease | The focus aid and the stack expose almost without a gap, so an in-flight check first answers `an exposure is in progress` where the lease would have named the holder (US6-S7). |
| The run's own imaging frames are taken at the exposure setting, uncapped, a known limit | The camera part asking 036's steering for the capped exposure before `hold` answers | 036 offers no getter for the capped exposure outside a stack's `before_exposure`, and adding one is an edit to 036. The smear falls on the corners and not on the solve's centre, which is all the run reads (`docs/research/2026-11-15-imaging-camera.md` § 7. Field rotation, sub length and the drive); the README and the field checklist say so (spec § Clarifications). |
| The time limit counts the clock from the start of stacking, pauses included | Counting only the time the stack is not paused | Its job is to end a forgotten run (spec owner decision 8). An unsteady tracker has no timeout of its own, and spec § Edge Cases tells the two apart. |
| The console's own mount, camera and solver parts land one story before the loop that re-centres | The loop first, on the script alone; both in one story | US4 proves the parts on the moving mount with runs that need no re-centre and with the mount part alone (US4-S5). US5 then adds the loop, and the wait a re-centre needs, and proves a re-centre end to end. In one story the two passed 64 KiB by the review's estimate; the loop first would have left a story of rig tests with no product code. |

## Dated notes and probe asides moved out of the trio

Each passage below was in the trio as quoted, with the file it was in before the
split; the trio now holds the same text without it.

- From spec.md:

  > where 036's tracker is not steady around the transit (about 312 s on the probe's code; plan.md § Prerequisites 2)

- From spec.md:

  > (032's FR-036; 032's owner decision 25, which owner decision 26 cites, settled on 2026-11-17). The re-centre's Track off puts it in force, and the live pointing the re-centre then waits for is the first taken at it (FR-025). As 032 was first drafted, the tracker chased it for up to the 5 s of a wait made with Track on (Clarifications).

- From spec.md:

  > The probe of round 1 did not see it: the simulated axes are at rest from the re-centre's Track off on, and Track itself waits up to 2 s for both axes to rest whenever it replaces a tracker, before it asks GoTo. Whether the real mount can be slower than that to stop is not established (plan.md § Risks).

- From spec.md:

  > (FR-029, US6-S7; changed by the owner on 2026-11-19, when this case paused the run).

- From spec.md:

  > (owner decision 9, as changed on 2026-11-19).

- From spec.md:

  > imaging frames too, which needs a getter 036 does not offer (Clarifications);

- From plan.md:

  > reaches into a GoTo, a re-centre or a take-up reads `moving`, or is biased: round 1 saw every solve after a re-centre take no aim reading for that reason. 036's `steady_between` is false

- From plan.md:

  > on 032 holds a newly measured aim point back (032's FR-036; 032's owner decision 25, which spec owner decision 26 cites, settled by the owner on 2026-11-17). Right after

- From plan.md:

  > measured aim point; during step 3 Track is off. As 032 was first drafted, the aim point was set at once and this wait came before Track off: the tracker took the whole step from the old aim point as its error and closed it at a rate nothing bounds (`crates/finder/src/tracking.rs:304`), under Track's own guards and outside GoTo's, for up to 5 s (spec § Clarifications). Since 038, a step of more than 5′ is read as a push by hand instead, which hands the tracker nothing but marks the pointing model's slip; a smaller one is still chased. The owner's answer to 032's decision 25 avoids both in 032. Probe round 1 confirmed on US5-S4 that the simulator's mount does not move toward the new aim point between the imaging solve and step 2, and that the model's slip is not marked, which since 038 is what tells the hold from no hold (§ Prerequisites 5).

- From plan.md:

  > replaces a tracker, before it asks GoTo, and the probe of round 1 saw the simulated axes at rest from the re-centre's Track off on. If

- From plan.md:

  > the stars lost in the corners is counted as any other (§ Failed steps and counts in the machine). This is a known limit (spec § Clarifications), which the README states and the field checklist's step 8 has the observer look for.

## Risks as they read before the split

Each row below was rewritten for the split to drop what probe round 1 had or had
not run, or, for the newly measured aim point under Track, which 032 has closed,
left out; plan.md § Risks holds what is in force.

| Risk | Mitigation |
|---|---|
| The simulator leaves the target just outside 90″, with the true aim point set or after a re-centre, or needs more than three tries. | Probed in round 1 (§ Prerequisites 5): 4.43″ by the truth with the true aim point set, and 8.67″ after one try with none. US4-S1 and US5-S4 assert the bounds against the truth, and owner decisions 2 and 3 are where the numbers change. The steady rule the fold wrote in the place of round 1's wait has not been run. |
| 032's aim reading fits the finder camera's solves from 10 s, or the exposure when that is longer, before the imaging exposure, so an imaging frame taken sooner than that after a GoTo or a re-centre reads `moving` and gives no aim reading. Round 1 saw it after every re-centre. | The mount part reads steady only when 036's tracker was steady for that whole look-back (`steady_between`, spec FR-019). The cost is the larger of 10 s and the exposure, after the 2 s to 3 s for which 036's tracker settles, after each GoTo and each re-centre. Not probed: round 1 built a fixed wait of 10 s instead, and no run had an exposure above 1 s; US4-S1's bullet and US5-S6, at 20 s, are for the second probe round. The lasting fix is a small follow-up to 032, which has landed: drop from the fit every finder solve older than the session's last end of a GoTo or start of Track. When it lands the look-back shrinks to `steady()`. |
| A push that reads `moving` for less than 10 s no longer ends the run, and while the run stacks nothing of this spec pauses the stack for it. | 036's tracker reads not steady while the push is `moving`, so the stack waits and the sub in flight is dropped as `not_steady`. Not run: 036's second probe round runs its US6-S6. US4-S4's bullet shows only that the run goes on. |
| The reserve is worked out from the files 035 writes at a stack's end as they are today: 15 bytes a pixel of a sub and a margin of 16 MiB, about 260 MB at the worked camera's full size (`reserve_bytes`; the owner's answer of 2026-11-19, in place of a fixed 64 MiB, which could not hold a full-size `stack.fits` of about 195 MB). | If 035 comes to write more at a stack's end, a fourth plane or a 16-bit picture, the figure for a pixel is one constant, and the re-anchoring reads 035's save again (§ Prerequisites 2). A `stack.json` that lists more than about fifty thousand subs outgrows the margin. The reserve helps only an ending the run makes: a stack that ends by itself saves while the reserve is still there. It is itself room the subs cannot use, and a disk that cannot hold it ends the run `save_failed` before its first sub. While it is written, 4 MiB at a time with a yield between, a stop of the run waits for `stack.start` to answer, for as long as the disk needs. Not run: the second probe round builds it (US8-S7). |
| A target changed in the instant after a re-centre has asked Track again is not seen by step 3a. | Track then goes on with the old target, the next `state()` reads `target_changed` by its second row, and Track is left on the old target, as outside a re-centre. US5-S7's timing rests on the first live pointing with Track off coming 0.30 s after it on the simulator; not run. |
| With `keep_subs` false, the two conditions under which the stack part removes subs are asserted on the script only, by the warning. | No rig scenario stops a run before its first sub is stacked at a moment a test can fix. The second probe round looks for one. |
| A newly measured aim point under Track: as 032 was first drafted, 015's tracker chased it at a rate nothing bounds, outside GoTo's checks; since 038, a step of more than 5′ is read as a push by hand instead and marks the pointing model's slip. | Closed in 032, where the seam between 032 and 015 lies: the owner settled 032's owner decision 25 on 2026-11-17 with the hold (032's FR-036, US4-S8), which spec owner decision 26 cites, and FR-022 and FR-025 here are written for it (§ Prerequisites 4). Probe round 1 saw that the simulator's mount does not move toward the new aim point before the re-centre's Track off (0.00″) and that the model's slip is not marked (§ Prerequisites 5), and the field checklist has the observer watch the first centring (step 8). |
| The observer's exposure setting is longer than the tracker's limit, and the run's own imaging frames, which 036 does not cap, trail their corner stars. | A known limit (spec § Clarifications; § The console's camera and solver parts). The solve's centre, all the run reads, is not moved; a solve that fails for want of stars is counted (US2-S2). The README says so and step 8 of the field checklist has the observer look. The other answer is a getter in 036 for the capped exposure, an edit to 036. |
| A re-centre asks Track again while the mount still reports motion after Track off; GoTo refuses with `the mount is already moving` and the run ends `mount_failed`. | Probed in 037's round 1: the simulated axes are at rest from the re-centre's Track off on, and `Session::start_tracking` waits up to 2 s for both axes to rest whenever it replaces a tracker, before it asks GoTo; GoTo did not refuse. The field checklist's step 8 still watches the first centring. `stop_tracking` itself waits for the mount's acknowledgement of the stop, not for rest (`crates/finder/src/tracking.rs:219`). If the telescope shows it all the same, `repoint` waits, for a counted number of polls, until the session's mount view no longer says the mount is moving (`crates/finder/src/session.rs:508`) before it asks Track; that reads no axis. |
| A second client stops or restarts the stack under a run. | The lease refuses a second stack; a stop of the run's own stack from 035's route ends the run `stopped` (US7-S4 on the script, US8-S4 on the rig). A stack another client starts in the instant of the hand-over, which 035's `start` joins and answers `Ok` for, is told from the run's own by its `started_at` and ends the run `stack_failed`, with nothing of that stack stopped or removed; that race was not run either. A stack started in the one poll between the run's stack ending and the run's own ending is left running; the run keeps its last counts and its subs (§ The console's stack part). That race was not run. |
| On the simulator no run meets 036's wait (`paused_by_tracker` on the console's own stack part), a stack's `camera` ending, a `save_error` at `stop`, or a dropped sub; and no test can see the set point. | Those five rows of the stack part are asserted on the script only (US2-S3, US2-S4, US7-S4). Round 1's mutations of each survived the rig's tests. |
