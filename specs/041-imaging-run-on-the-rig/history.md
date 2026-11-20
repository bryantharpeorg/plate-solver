Not given to builders or the judge. Nothing here is a rule; the trio is.

# How 041-imaging-run-on-the-rig got here

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
