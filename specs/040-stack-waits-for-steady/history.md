Not given to builders or the judge. Nothing here is a rule; the trio is.

# History: The Stack Waits for a Steady Tracker, and Shows the Steering

How `040-stack-waits-for-steady` came to be as it is. The trio holds the rules and
flip.md what the flip settles.

## The split (2026-11-20)

This spec was cut from `036-tracking-from-subs` on 2026-11-20, when that spec held eleven
stories:

| Piece | Stories | FRs | `depends_on_landed` |
|---|---|---|---|
| `036-tracking-from-subs` | US1, US2, US3, US9, US4, US5 | FR-001 to FR-020, FR-030 | unchanged |
| `040-stack-waits-for-steady` | US6, US10, US7, US8, US11 | FR-021 to FR-029, FR-031 | 036's, and `036-tracking-from-subs` |

Every story and FR number was kept. Each story went whole to one piece, with its
scenarios, FRs, interfaces, Refinement Record block, tasks phase, edge cases and traps. A
section that served both stayed in 036, and 040's plan.md § What 036 provides lists what
040 uses of it as landed. 040's Work Graph dropped US6's edges to US4 and US5 and US7's
to US5, which are 036's and land first. A cite across the pieces reads "040's US6-S5"
or "036's US4-S12". History left the trios for history.md, and what the flip settles for
flip.md.

Everything before the split, the drafting, the probe rounds, the reanalysis and its
folds, the Clarifications and the old comment header, is in 036's history.md, which also
has a table of where each section went.

## Story notes moved out of spec.md

**User Story 6**, under its story:

The cases of the reanalysis's second pass took this story's estimate past 85% of the
64 KiB budget, so it was cut as User Story 3 was (the owner's choice 14 of 2026-11-19):
eight scenarios of User Story 10 are US6-S11 whole and the later halves of US6-S1 and
US6-S2, which each held more than one Given, When and Then. What they test is still this
story's code. The scenario that was US6-S12 took the number 11, and the later halves of
US6-S3 are US6-S12 to US6-S16 (§ Clarifications, the session of the reanalysis).

**User Story 10**, under its story:

This story is built after User Story 6 and before User Story 7 (Work Graph). It has the
number 10 because it was cut from User Story 6 on 2026-11-19: the cases of the
reanalysis's second pass took that story's estimate past 85% of the 64 KiB budget, and
it was cut as User Story 3 was (the owner's choice 14). Eight of its nine scenarios were
US6-S11 whole and the later halves of US6-S1 and US6-S2, which each held more than one
Given, When and Then; the ninth is a case the reanalysis proposed that no scenario held
(§ Clarifications, the session of the reanalysis, has the table). What they test is User
Story 6's code: this story writes nothing under `crates/finder/src`, and on User Story
6's code its scenarios already pass (tasks.md, US10's test tasks). One of them, US10-S8,
has no bound yet: probe round 2 measures it.

**User Story 11**, under its story:

This story is the alternative of owner decision 11, taken by the owner on 2026-11-19
(the reanalysis, choice 12). It is built last (Work Graph), after the page's story,
because it edits the README's list of tools after that story's section.

## Asides moved out of the trio

Each was replaced in the trio by the rule or the reason it gave, without the date,
the probe round or the earlier draft it told of. As each read:

- **spec.md US6-S8, its oracle**:

  > that state says takes the rate off the mount (oracle: the rig's rate log. The recheck
  > of round 1 measured +25.3″/s still in force at such a gap's answer, for a pointing
  > with no earlier one and no settling).

- **spec.md US7, the stacking rig**:

  > imaging view's `state` is `ready`. (Round 1's rig waited 1 s more here, D38. It no
  > longer needs to: the tracker settles for 2 s after its first rates, the stack's first
  > `before_exposure` waits that out, and a sub that starts at the instant the tracker
  > became steady is kept: plan.md § The tracker's two answers.) **Waiting until** a

- **plan.md § Steady, A push by hand**:

  > so as this trio first had it the tracker
  > read steady with a hand on the tube: exposures went on, every sub taken across the push
  > was kept, and a sub taken after the hand had left and before 038's five still seconds
  > were up was accepted with the whole push as its error, which the next gaps stepped back
  > toward the old place at 30″ a gap.

- **plan.md § Steady, the window's end**:

  > Round 1 built
  > both bounds inclusive, and a stack started at the instant of a tracker's first rates
  > lost its first sub (D38). With 2 s

- **plan.md § Steady, the window's end**:

  > is not counted and the stacking rig of spec.md US7 no longer waits a second more.

- **plan.md § The steering's wait and its check, `dropped_not_steady`**:

  > As drafted it counted since the console started,
  > and from a console's second stack on it disagreed with the count shown beside it
  > (reanalysis FF18, whose own fix cleared it in `release`; the two would then have
  > disagreed after every stop).

- **plan.md § The stack, its opening**:

  > (read on 2026-11-19;
  > round 1's probe built on an earlier 035, whose worker counted in numbered steps); each

- **plan.md § The stack, the worker's drop**:

  > `LastView` does not carry it. Round 1's words for this drop, "counted inline, as the
  > settings drop is, and not through `Dropped::count`, whose match sends a key it does not
  > name to `unprocessed`", were true of the probe's 035 and are false of the one that
  > landed (reanalysis BB1, CC10, DD4, EE3; BB2 for the place of `accepted`).

- **plan.md § The stack, The guard**:

  > The last seven
  > were added on 2026-11-19: with six, a file could have said `azimuth` or `keyhole` and
  > passed (reanalysis FF20).

- **plan.md § The page, its words**:

  > They were reworded on 2026-11-19: as
  > drafted one sentence said `Track is off` while Track was armed or had ended by itself
  > with a stack still exposing, one covered four states of which only one needs the
  > observer, and a pause gave no cause and no end (reanalysis FF16).

- **plan.md § The page, the settings' drafts**:

  > Round 1 saw
  > 031's exposure field lose a typed value

- **plan.md § Risks, 035 changes before it lands**:

  > lists the nine points this spec leans on, which probe round 1 found in 035's probe code with D39 to D41's words, and

- **tasks.md T028**:

  > queued behind a take-up when tracking ends (US6-S10). Nine scenarios on this rig are
  > US10's since 2026-11-19.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Not steady drops the claim | Steering through a take-up | 015's handling of a reversal and of the keyhole is tested as it stands; the subs come back once it is over. |
| A push the session reads as moving is a time the tracker is not steady | Steering stopped and exposures going on; as drafted, with 037 ending its run on any push | 038 leaves a push alone until it has settled, and a running stack did not: it stepped the telescope back at 30″ a gap. Not steady, one mechanism pauses the exposures, drops the smeared sub and the claim, and gives 037 the wait its own design assumes (spec owner decision 17, changed 2026-11-19). |
| 2 s of settling after a tracker's first rates, a take-up and the keyhole pass, kept by the tracker | None; a look-back in the steering's wait | Steady was commands acknowledged, not a tube at rest. Kept by the tracker, the page, 037 and `steady_between` all see the same time; a wait after a large step in a gap was not taken (spec owner decision 17, changed 2026-11-19). |
| The tracker reads not steady for a turn that finds the azimuth axis at rest after a rate that was not zero | A field check only | The scope node stops both axes on a failed rate command and still acknowledges it; the trailed sub was kept and its error steered. The node's own fix is another spec (spec owner decision 17, changed 2026-11-19). |
| `check` before the solve, `accepted` once the worker has accepted the sub, right after `note_solve` | One call after the solve | A sub taken across a take-up is smeared: solving it costs time and would move the stack's own hint. The tracking error must wait for the stack's verdict, and must not wait for the picture 035 may render after it. |
| The cap applied by changing the console's exposure setting through the stack's own lease, and put back through it as the stack ends | A second exposure parameter on the take; 034's `Calibration::restore_exposure` | 031's take and its refusals are pinned by 031's tests, and 035's seam for 036 has a stack's exposure set through the taker's lease (035 plan.md § Shapes that existing tests pin, **The seam for 036 and 037**, 2). `restore_exposure` called `Imaging::set`, which the stack's own lease refuses, so the subs would have been taken at the observer's exposure; 034 has since removed it (034 A1). |
| A stack that cannot set its capped exposure ends with `camera` | Taking the subs at the observer's exposure and saying `over_limit` | The view would show the exposure held while longer subs were taken; and a refusal there means the camera is no longer ready. |
| Not steady runs 015's corrections whole, even after a lapse | Holding them through a time the tracker is not steady | Gaps are turned away while the tracker is not steady, so a held correction would wait for it to end, and 015 closes a first reversal's measurement from its own corrections, which would then never come (spec owner decision 17). No sub the stack keeps is exposed then. |
| The taker puts the observer's exposure back as it returns | 035's ending, after the taker has returned | The taker owns the lease, which drops as it returns: after that there is no lease to set through, and another holder may already have the camera. 035's seam says the same. |
| Not steady while a first reversal's backlash is measured | Steady, with the subs steering through it | 015 closes that measurement from the finder camera's pointings and its own corrections; changing what steers during it would need 015's backlash tests proven again (spec owner decision 17). |
| A wait while the limit is under half the exposure held, the tracker let go for it | Stacking on with `over_limit` shown; ending the stack | The stack never stacks subs smeared by more than twice the smear chosen, and goes on by itself once the limit has risen, which after a transit it does. Let go, the tracker's own corrections steer through a wait that can last many minutes; held, nothing would. |
| Why a tracker is not steady as the tracker's own word, through `Steered::pause` and the view | One sentence for every pause; the steering naming the causes | The causes last from under a second to a quarter of an hour, and only a push needs the observer. The words are a mount's; kept in the tracker and the page, the imaging path names no axis. |
| A word of its own, `stalled`, for a turn that finds an axis stopped | No word, the page's general sentence serving | The other pauses each say why; this one is the only sign on the page that the node stopped the mount. It lasts under the steering's poll, so it is seldom seen, and the README and the node's journal say the rest. |
| `dropped_not_steady` set to 0 where a stack takes its `Hold` | Since the console started; cleared in `release` | It sits beside 035's count of the same drops, which is one stack's and stays after the stack has ended. Cleared in `release`, the two would disagree after every stop. |
| `set_steering` as a story of its own, built last | A part of US5; a part of US8 | Spec owner decision 11 named it "one more story after US5". US5 is the second largest story with this fold's cases, and a tool, its schema and a module in a third test binary are about 9 KB more. |
| `get_imaging_camera`'s description left as 031 has it | Naming the stack and the steering in it | 031's MCP test asserts its words, and no existing test is edited. |
