Not given to builders or the judge. Nothing here is a rule; the trio is.

# How 037-imaging-run got here

## The re-anchoring and the flip (2026-11-20)

The trio was re-anchored onto `main` at `6d79292b` once 040 had landed (#336 to #340).
Of the 20 code citations the draft held at 49bb210, 13 were where they had been, 6 had
moved and were fixed, and 1 was found again by its content; 14 more were added for code
that has since landed. Every name the trio takes from 030 to 036 and 040 was read in the
landed code, and none differs in a way that changes what a story builds. The one wording
written for the landed code is that US3 sets `run` after 035's `stack` and 036's
`steering`, the view's last part, as the probe built it. The five shared writes that
name another spec matched `git log -1`. Principle V's row was added (§ Decisions and
the alternatives rejected).

043 landed the same day (#342, `2ae73c66`), before this spec was flipped, and the trio
was carried across it:
- `043-imaging-abort-answers-in-time` is in `depends_on_landed`;
- the five citations of `crates/finder/src/imaging/mod.rs` moved two lines down, past
  043's `ABORT_WAIT`; no cited line was one 043 changed;
- US12's shared writes of `imaging/mod.rs` and of the `console` test binary's `main.rs`
  name 043's US1, which last wrote both, where they had named 036's US5 and 040's US8.

This spec reads nothing of 043's.

The seat flipped the trio to `ready` the same day, under the owner's standing permission
for the imaging specs (flip.md § Prerequisites 4). Every owner decision takes the answer
flip.md writes: the recommended answers the owner approved on 2026-11-17, with his
changes of 2026-11-19 to 1, 7, 9, 14 and 22. Owner approval 8 gained
`imaging_run_script`, the module US13 was cut with, so that it names all five test
modules SC-002 names.

## Probe round 2 and its fold (2026-11-20)

Probe round 2 built this piece's four stories, as the split of 2026-11-20 left them,
on 2026-11-19 and 2026-11-20: one agent built US12 and US1, a second US2 and US3, each
starting where the one before stopped, on 036's round-2 probe build, tests first, with
both gates and from the trio alone (flip.md § Prerequisites 5 says what it ran). Every
story built, passed both gates and wrote only its declared files. The round found 0
blocking and 15 clarity defects, 037-R1 to 037-R15, and they were folded on 2026-11-20,
while 040 was building and before the re-anchoring onto `main`, which is a step of its
own. Where a scenario gained a case, it has the words of the throwaway test the probe
ran, which passed on the probe's code and failed under the mutation it was written for.
037-R8 replaces 037-R4's fix: R4's case holds the run at `first_frame`, not at
`stacking`.

| Defect | What it found | Where it went |
|---|---|---|
| 037-R1 | Nothing in US1 constructs seven of the nine warnings, so a private enum of them fails `clippy -D warnings`, and the plan named no public item for them | plan.md § The machine, a public `Warning` with `words`; tasks.md T003 |
| 037-R2 | US12 built at 51,369 B, 18.4 % over its estimate; at § Risks' 1.35 it would pass 64 KiB | US12 cut by tests into US13 (below); plan.md § Risks |
| 037-R3 | No test pins `limit_at` to the request's `duration_s` (M5 survived) | a third bullet of US1-S2 |
| 037-R4 | No scenario reaches a stop of the run that finds Track off (E6 survived) | a case of US2-S7, held at `first_frame` as 037-R8 rewrote it |
| 037-R5 | The record's shared writes named 036's US4 for `imaging/mod.rs`, and five reasons read "whose 041's US<n>" | plan.md § Refinement Record: 036's US5, and "whose US8" or "whose US5" |
| 037-R6 | US1 needs `use` lines US12's lack, and "changes none of that" read strictly forbids them | plan.md § The machine and tasks.md T003: the `use` lines are no item |
| 037-R7 | Who words `SubsFailing`'s reasons was not said | plan.md § The run's words: `message` inserts them as the variant holds them; 041's US7 words them |
| 037-R8 | `guarded` and `steady` take a stop before the mount's ending and step 6 the other way, with no scenario on step 6's order (F32), so R4's case on a stacking run no longer bites E6 | R4's case at `first_frame`; a case of US2-S8; plan.md § Failed steps and counts, step 6 |
| 037-R9 | Three equal failure words leave "the last error" unpinned (F7) | US2-S1's second bullet: `boom`, `boom`, then `gone` |
| 037-R10 | No scenario reads `saved` after a failed `stack.stop` (F35) | US2-S4's third bullet: `ended.saved` false |
| 037-R11 | The wait's words are never seen to go (F31) | US2-S5: `solve` held too, the message read after the wait |
| 037-R12 | The counts read after `stack.stop` have no oracle, and US3-S3's cannot be one (F28) | US2-S4's last bullet; US3-S3 drops "and not the 4 of the last poll" |
| 037-R13 | US3-S11's 60 s cannot tell the rule from holding back by the last change held back (N25) | US3-S11 sleeps 58 s, and its Then clause says why |
| 037-R14 | US3-S9's one record cannot tell the newest from the oldest (N27) | US3-S9 gains an older record that ended |
| 037-R15 | 036's and 040's plan.md point at this plan's table for five items it no longer lists | not folded (below) |

- Q: What did 037-R2 come to, under owner choice 14? → A: A cut by tests. The owner took
  choice 14 on 2026-11-19: "if the fold's estimate passes 60 KB, cut it by tests into a
  new story". The night's factory builds ran 0.93 and 1.04 times their probes' diffs
  (039's US1 and US2), 0.94 to 1.17 (036's stories) and 0.85 and 0.99 (040's US6 and
  US10), so the fold took the highest, 1.17, and not § Risks' 1.35. No folded defect adds
  a case to US12, so its estimate is 51,369 B × 1.17 = 60,102 B, 102 B over 60,000 B.
  US12-S3 and US12-S4, the script's defaults and call words and its controls, are
  User Story 13's, with their module `imaging_run_script`; the code, `script.rs`
  included, stays in US12. By the probe's bytes by scenario the two are 2,859 B and
  7,659 B of US12's test diff, so US12 is at most 51,369 − 2,859 − 7,659 = 40,851 B,
  47,796 B at 1.17. US13 is their 10,518 B with the module's head of 607 B, the helpers
  `m42` and `frame` of 296 B (US12 keeps `m42` too), a file header of about 250 B and
  its `mod` line of about 385 B: about 12,056 B, 14,106 B at 1.17. The other stories at
  1.17, with the bytes of the throwaways their cases add: US1 (33,997 + 456) × 1.17 =
  40,310 B, US2
  (34,869 + 3,201) × 1.17 = 44,542 B and US3 (47,736 + 1,266) × 1.17 = 57,332 B, the
  story to watch. US13 `depends_on_merged` US12, implements no FR and writes nothing
  under `crates/finder/src`, as 036's US12 does for its US5; US1 names US12 and US13,
  since both last write a file it writes, so the script is proven before the machine is
  written on it. In the record US12's shared write of the `console` binary's `main.rs`
  names US13, and US13's names US1. tasks.md's T046 is no longer used; US13's tasks are
  T050 and T051. No case was dropped or shortened.

  | Was | Now | Module |
  |---|---|---|
  | US12-S3 (the script's defaults and call words) | US13-S1 | `imaging_run_script` of `console` |
  | US12-S4 (the script's controls) | US13-S2 | `imaging_run_script` of `console` |
- Q: Why was 037-R15 not folded? → A: Its fix texts are edits of 036's plan.md and 040's
  plan.md, and this fold edits neither: 036 has landed, and 040 is building. Neither
  touches a build of this piece, which reads only the constant `FAR_ARCSEC` of 036's and
  nothing of 040's. It is left for those trios' own owners.
- Q: What did the owner answer on the pause line? → A: On 2026-11-20, "for 037 agree with
  your recommendation": the run's `paused_by_tracker` line names no cause, and no field
  is added to `StackProgress`. The trio already said so (plan.md § The run's words,
  "`paused_by_tracker` names no cause"; flip.md § Prerequisites 2), and nothing changed
  for it.
- Q: What else did the round note, and what was left? → A: Three notes that are no
  defect, none folded: W6, the one survivor of US12's 33 mutations (`label` written as
  a space made `-` passes, since US12-S2's worked example has only a space; the probe
  proposed `label("Sh2-155 (Cave)")` as a further example); `RECORD_EVERY`, written
  inside "`Runner` gains" and built as `Runner::RECORD_EVERY`, which no test reads; and
  a stop arm of a `select!` that does not return in the same turn, which spins the
  paused clock, for 041's US7 to keep in mind. The stale anchors the round found are the
  re-anchoring's (flip.md § Prerequisites 5). No other trio cites US12-S3 or US12-S4:
  041's and 042's drafts cite US12-S1 and "037's US12" for the script, both unchanged.
  Owner approval 8 in flip.md still names four test modules; `imaging_run_script` is to
  be added to it at the flip, with the rest of the flip's wording.

## The split (2026-11-20)

On 2026-11-20 the owner approved cutting the imaging run's trio, then `037-imaging-run`
with twelve stories, into three specs along its serial Work Graph, and moving its history
out of the trios ("I approve the recommendations you made to slice the specs and move
some stuff out"). Ergane hands every attempt of every story the whole plan.md, and 037's
was 273 KB; and every review read the whole trio, where one rule was often stated in
several places.

| Spec | Stories | What it holds |
|---|---|---|
| `037-imaging-run` | US12, US1, US2, US3 | the parts' traits, the words and the script; the machine with its retries, failed steps and counts; the routes and the record; all on the script |
| `041-imaging-run-on-the-rig` | US4, US5, US6, US7, US8 | the console's own five parts; centring, the focus gate and its pause, the limits and the stack's own endings; the binary's wiring |
| `042-imaging-run-page-and-mcp` | US9, US10, US11 | the run block on the Imaging page, the four MCP tools, and the README with the field checklist |

The cut follows the build order, so each piece lands before the next starts. Every story,
scenario and FR kept its number, and a cite from one piece to another names the piece
(`041's US6-S1`). Each story went with its scenarios, FRs, interfaces, tasks phase,
edge cases, traps and Refinement Record block. Text that served stories of two pieces
stayed with the earlier one, and the later piece's plan.md has a section `## What
<earlier spec> provides (read as landed before the flip)`: 041 one for 037, 042 one for
037 and one for 041. Edges of the Work Graph to a story now in an earlier piece were
dropped and that piece added to `depends_on_landed`: US4's edge to US3, and US3 from
US5's and US8's; US8 from US9's and US10's, and US7 from US11's. In the Refinement
Record a story's shared write naming a later story now in another piece was dropped, and
the first writer of a file in 041 or 042 names the last writer of it in the piece
before: 041's US4 and US5 name 037's US3 for `run.rs` and the `console` test binary's
`main.rs`, 041's US8 names 037's US3 for `api.rs`, 042's US10 names 041's US8 for
`api.rs`, and 042's US11 names 041's US7 for the `console` binary's `main.rs`.

`036-tracking-from-subs` was split the same day into 036 and `040-stack-waits-for-steady`.
Each cite in the three trios of a 036 story or FR that went to 040 now names 040, and
040 is in each piece's `depends_on_landed`.

Each piece has its own history.md and flip.md. What serves the whole run stayed here:
the drafting header, the Clarifications, settled owner decision 26 and the Refinement
Record's prose. The owner decisions still open and the owner approvals are in the
flip.md of the piece whose stories they serve, under the numbers they had; approval 8,
the `mod` lines, and approval 9, nothing else new, are in each, as each piece's SC-002
and SC-003 now state them.

Below § The split, the text is as it stood before the split: its story, scenario, FR,
decision and section numbers are those of the twelve-story trio, every one of which kept
its number. A story of US4 to US8 is now in `041-imaging-run-on-the-rig`, one of US9 to
US11 in `042-imaging-run-page-and-mcp`, and an owner decision in the flip.md of the spec
that owns it; a story of 036 that went to `040-stack-waits-for-steady` is cited here as
036's.

## The drafting header

The comment header of spec.md before the split, as it stood:

```text
Drafted 2026-11-15 against `main` at 49bb210 (the drafting worktree's HEAD,
49bb210fe4860dd98ba24727074e7bd3cc0d7124) from
docs/plans/2026-11-15-specs-030-037-imaging.md, by the grilling session of that day.
A first draft was written against dbc2b5b before 033, 035 and 036 had drafts and
before the plan document gained "Mount-neutral by design" and its amendments; this
text replaces it and is in line with both.
When drafted it was not probe-built: no story of this trio had been compiled or
run (plan.md § Refinement Record). It was drafted on code that 030 to 036 create
and that did not exist then. Their interfaces were taken from their plan.md in
the same worktree, name for name: the lease of the imaging camera, which the plan
document's amendment 1 gives
031, from 031's US9, and the rule for a stack's folder name from 035. The trio is
re-anchored to `main` after its seven predecessors land and before the flip.
A second agent reviewed the trio the same day; its twenty-two findings are answered
in this text, which then had eleven stories where the reviewed draft had eight. An audit
of the seven trios the series then had, side by side, then gave it four more (A1 to A4), also answered
here (the last two sessions under Clarifications; plan.md § Refinement Record).
Citations checked against 49bb210 (49bb210fe4860dd98ba24727074e7bd3cc0d7124): after
the audit every `path:line` in spec.md and plan.md was read again at that commit.
It stays draft until the owner settles the owner decisions and flips it to `ready`,
which gives the approvals listed under "Owner approvals" and dispatches it.
Probe-built 2026-11-19 (round 1): all eleven stories were built from this trio, tests
first and with both gates, on `main`'s 030 to 034 and 038 and on probe builds of 035
and 036. Its 3 blocking and 35 clarity defects, and the 71 points that six reviews of
the round then raised, were folded in on 2026-11-19 with the owner's answers of that
day ("Take all 17"). The answers change what ends a run, what a run keeps and what
the observer is told; US1's tests of a stop, the mount's endings and the start's
refusals are now US2's; and the trio had 90 scenarios where the probed draft had 58
(§ Clarifications, the session of round 1 probes and the reanalysis; plan.md
§ Prerequisites 5 and § Refinement Record, "What probe round 1 and the reanalysis
changed"). Nothing the fold added has been run: a second probe round builds the
changed stories before the flip.
After the fold, the same day, the owner split US1 in two ("Yes, split US1"): a new
US12, built first, holds what a run is handed and what it can say with no run, the
five parts' traits, the endings with their words, the request and the script, and
US1 keeps the machine and what only a run can show. He sized the stack part's
reserve to the stacked picture ("Yes, size it"), and the pause after a focus check
that did not finish got words of its own. The trio has twelve stories and 93
scenarios (§ Clarifications, the last question of that session). The split has been
built by no probe.
```

## Clarifications

### Session 2026-11-15 (the interview's decisions, from the plan document)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  binding decisions come from `docs/plans/2026-11-15-specs-030-037-imaging.md` (the plan
  document), `docs/adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md`
  and `GLOSSARY.md`; facts come from `docs/research/2026-11-15-imaging-camera.md`. A
  story that would change a decision escalates rather than re-deciding it.
- Q: Who runs the procedure? → A: The console. It owns one imaging run at a time. The
  page's button and one MCP tool start the same run; an MCP client chooses, steers and
  stops. Each step keeps its own route and MCP tool from 030 to 036, so every step can
  still be done by hand.
- Q: How does the run reach the mount? → A: Through three of the console's own actions
  and nothing else: Track on for the session's target, which starts a GoTo first when
  the target is more than 1′ from the aim point (`crates/finder/src/session.rs:809`,
  `crates/finder/src/session.rs:845`); Track off; and Stop. It reads two answers back:
  whether the mount is still getting there, and whether the tracker is steady enough to
  expose. No file of this spec sends an axis command, reads an axis position or computes
  an altitude, an azimuth, a field rotation or a rate (the plan document,
  § Mount-neutral by design, rules 1 and 5).
- Q: In what order do the steps come? → A: Slewing, tracking, a first imaging frame, the
  focus check, the imaging solve, centring, stacking, an ending. The focus check comes
  before the first imaging solve because an unfocused imaging frame may not solve
  (`docs/research/2026-11-15-imaging-camera.md` § 12. Prior art and what was not
  adopted; § 3. ASTAP).
- Q: What does the focus check do? → A: It is 033's focus check, which takes its own
  short exposures of the focus region and ends with a verdict against the focus record.
  Good goes on. Slightly soft warns and goes on. Clearly out, or unmeasurable, pauses
  the run and opens the focus aid. The observer can always go on anyway. No absolute
  "focus is good" is claimed (`docs/research/2026-11-15-imaging-camera.md` § 6. Focus).
- Q: How is the target centred? → A: The first imaging solve of a night searches wide,
  and 032 measures the aim point and the imaging angle from it. The run compares the
  solve's centre with the target. Outside the tolerance, it turns Track off, which puts
  the newly measured aim point in force (032 holds it back while Track is on; owner
  decision 26), and Track on again, which brings the target to it through GoTo; then it
  takes another imaging frame and solves it, at most three times. It never nudges an
  axis (rule 5).
- Q: What happens during stacking? → A: 035's stack takes and places the subs and 036's
  steering corrects the tracker from them; the run starts the stack, watches its counts
  and stops it. The run adds no rule of its own about exposure length: the exposure is
  the imaging camera's setting, which 036 caps for the stack's subs by the tracker's
  first answer. The run's own imaging frames are not capped (a known limit, in the
  last session below).
- Q: How does a run end? → A: By the observer (the run's own stop, or the console's
  Stop), at a limit (a number of subs or a time), when a guard of Track or GoTo ends the
  motion (target too low, Sun up, link lost), on a camera failure, when the imaging
  solver cannot run or keeps failing, when the console cannot save, when a focus
  pause is left too long, or when the observer pushes the telescope away by hand
  (038; owner decision 27). Every ending stops the stack, which saves it, and leaves the
  mount as the guard that fired left it.
- Q: Who holds the imaging camera during a run? → A: One holder at every moment. The
  plan document's amendment 1 gives the console's imaging state one lease that names its
  holder, which 031 writes as its US9, and a second taker is refused with the holder's
  name. The run takes the lease,
  as `the imaging run`, before it asks for Track, and holds it while it slews, takes its
  own imaging frames, solves and centres. 033's focus check and focus aid and 035's
  stack each take the lease under a name of their own, so the run hands it over: it
  gives the lease up just before a focus check and takes it again when it goes on to
  the solve, and gives it up just before the stack is started. It adds no lock of its
  own (owner decision 15).
- Q: Is anything keyed to one telescope, mount or camera? → A: No. Thresholds are
  arcseconds or shares of the imaging frame, counts are in subs or imaging frames, and
  the figures in seconds are a poll, a bound on the finder camera and two waits for a
  person (below; the owner's answers of 2026-11-19 added five more, listed there).
  Scale comes from the imaging solve. The owner's telescope appears only
  as a worked example (rule 3).
- Q: Rule 6 says nothing is "fixed in seconds". Which figures here are in seconds, and
  why may they be? → A: Four, none of them a wait for a sub or an exposure. The run
  reads the mount and the stack every 250 ms: a poll, not a timeout. A re-centre waits
  at most 5 s, twenty of those polls, for the finder camera's next live pointing: a
  bound on the finder camera, not on the imaging camera, and the interval within which
  015's tracker already expects a live pointing
  (`crates/finder/src/tracking.rs:248`). A focus pause ends after 15 minutes: a wait
  for a person at the focuser. The time limit of a run is the observer's own setting, 2
  hours unless they give another. Every other bound is a count: three imaging frames,
  three solves, three centring tries, ten subs.
  **Changed by the owner on 2026-11-19** (the answers to the reanalysis of round 1's
  probes): the figures in seconds are now nine, still none of them a wait for a sub or
  an exposure. The five added are these. The mount part reads steady only when 036's
  tracker was steady for the whole of 032's fit reach before now, 10 s or the exposure
  setting when that is longer: 032's own look-back, not a wait of the run's (FR-019).
  A push by hand ends the run only once it has lasted 10 s or has settled somewhere
  else (FR-023). A failed take is asked again 5 s later: a wait for a link to come
  back (FR-009). The run ends `subs_failing` after 600 s of stacking in which a sub
  failed and none was stacked, which replaces the count of ten subs (FR-031). And
  `run.json` is refreshed at most every 60 s while the stacked count rises, and lists a
  change between `stacking` and `paused_by_tracker` at most once in the same 60 s
  (FR-017).
  The page has one more of its own, the 5 s in which a second tap confirms
  `Stop imaging run` (FR-035). The counts are now three imaging frames, three solves,
  three centring tries, and four solves in a row that measure no new aim point.
- Q: Where does the run want the target? → A: At the set point, the centre of the
  imaging camera's sensor. The run says so to 036's steering when it starts the stack,
  and measures centring against the same place. A later run that names another place
  changes those two lines and nothing in the stack (rule 7).

### Session 2026-11-15 (review of `main` at 49bb210, and of the drafts of 030 to 036)

- Q: What was found in the code? → A: Nine things, each checked at 49bb210:
  1. Track is one function, `Session::start_tracking`
     (`crates/finder/src/session.rs:809`). It refuses with the planner's words when
     there is no mount, sync, site or pointable target or the Sun is up
     (`crates/finder/src/session.rs:782`), starts a GoTo when the latest pointing is more
     than 1′ from the target, reports `armed` until that GoTo is `done`, and ends armed
     with the GoTo's own reason when it fails or halts
     (`crates/finder/src/session.rs:893`). With Track already on for the same target it
     answers at once and changes nothing.
  2. A GoTo arrives when the live pointing at the aim point is within 60″ of the target,
     and fails after five corrections (`crates/finder/src/goto.rs:1013`,
     `crates/finder/src/goto.rs:1018`). So nothing the run does through GoTo can centre
     more finely than 60″ at the aim point.
  3. The tracker ends itself as `stopped`, `link_lost`, `sun_up` or `too_low`, or with a
     motion failure's words (`crates/finder/src/tracking.rs:61`,
     `crates/finder/src/tracking.rs:382`), and a GoTo halts with
     `the observer stopped it`, `halted by a lost link` or `halted by the Sun guard`
     (`crates/finder/src/goto.rs:848`, `crates/finder/src/goto.rs:813`,
     `crates/finder/src/goto.rs:817`).
  4. Every stop goes through `api::stop` (`crates/finder/src/api.rs:904`), which ends
     tracking and calls `GoTo::stop`; that advances a stop generation before it sends
     anything (`crates/finder/src/goto.rs:842`), and `GoTo::stops` hands out a receiver
     of it (`crates/finder/src/goto.rs:801`). The page's Track toggle turned off is the
     same Stop (`crates/finder/src/api.rs:808`). `Session::stop_tracking`
     (`crates/finder/src/session.rs:950`) ends tracking without advancing it.
  5. Who moved the mount is recorded by `start_motion`
     (`crates/finder/src/api.rs:593`), which holds the motion lock while the start runs
     and shows `motion: {by, tool, since}` while a GoTo, Track or direct axis move runs
     (`crates/finder/src/api.rs:697`). It refuses an MCP `set_tracking` while a direct
     axis move runs (`crates/finder/src/api.rs:623`).
  6. An MCP moving tool takes the rate limit first and writes one call-log line
     (`crates/finder/src/mcp/tools.rs:216`, `crates/finder/src/api.rs:305`,
     `crates/finder/src/api.rs:322`); the door carries out `stop` and `stop_all` before
     any check (`crates/finder/src/mcp/door.rs:33`).
  7. The page shows `Moved by an MCP client · {tool}` while `motion.by` is `mcp`
     (`crates/finder/web/src/components/McpNote.tsx:5`), beside the top bar's Stop
     (`crates/finder/web/src/components/TopBar.tsx:128`).
  8. A test clock already lives in the library, `FixedClock`
     (`crates/finder/src/session.rs:41`), and finder's `layout` test fails any new file
     in `crates/finder/tests/support` (`crates/finder/tests/layout/main.rs:127`).
  9. One existing test calls every listed MCP tool with empty arguments and requires that
     no answer holds the node's token (`crates/finder/tests/mcp/motion.rs:528`).
- Q: Why can the run not ask GoTo directly to re-centre? → A: While Track is on the mount
  is moving, and GoTo refuses with `the mount is already moving`
  (`crates/finder/src/goto.rs:739`). The console's own way to bring the target to the
  aim point again is Track off and Track on; the run does exactly that (FR-020), and
  Track decides whether a GoTo is needed.
- Q: A latest pointing is taken at the aim point in force when its star frame was solved
  (`crates/finder/src/session.rs:1192`). What does that mean for a re-centre? → A: While
  Track is on, 032 holds a newly measured aim point back, and the pointing the session
  holds was taken at the old one; Track's 1′ test would read it. 032 applies the held
  aim point at the first live solve the session records with Track off, before that
  solve's pointing is taken (032's FR-036). So after it turns Track off the run's mount
  part waits for the finder camera's next live pointing, for at most 5 s, and only then
  asks Track again (FR-025). After 5 s it asks Track anyway; at worst a try is spent.
- Q: What does 015's tracker do with a newly measured aim point meanwhile? → A: As 032
  was first drafted, it chased it, and nothing in this trio could stop it. While Track is
  on, every live pointing reaches the tracker as the target's offset from the aim point in force
  (`crates/finder/src/session.rs:972`). The tracker turns an offset beyond its 10″
  deadband into a rate that removes 60% of it within the interval to the next solve,
  and nothing bounds that rate: only its slow integral term is clamped
  (`crates/finder/src/tracking.rs:304`, `crates/finder/src/tracking.rs:310`). So the
  first live pointing taken at a newly measured aim point hands the tracker the whole
  step between the old aim point and the new one. On the simulator of US5-S4, where
  the centre of the finder camera's frame, (728, 544), lies about 127 pixels, some
  3800″, from the true aim point, that is 0.6 × 3800″ over one interval between finder
  solves: about 1.3° a second at two solves a second, along the direction of tracking
  (a correction against it is held at zero). That motion is Track's, not GoTo's. It passes Track's own guards
  (the minimum altitude, the Sun, the link) and none of the checks GoTo makes along a
  path. It would have lasted from 032 setting the aim point, inside the run's imaging
  solve, until the run's own Track off: at most the 5 s the first draft waited with
  Track on. It is a seam between 032 and 015, not of this spec: it arises without a run
  too, whenever an imaging solve measures a new aim point while Track is on, as under a
  stack started by hand (035), so it is reachable from 032 on. It is 032's owner
  decision 25, which this spec's owner decision 26 only cites. The owner settled it
  there on 2026-11-17 with 032's recommended answer, written as its FR-036 and US4-S8:
  032 holds a measured aim point back while Track is on and applies it at the first
  live solve the session records with Track off, so the tracker is handed no such step.
  Since 038 landed on 2026-11-17, such a step is no longer chased when it is more than
  5′: it is read as a push by hand, which hands the tracker nothing but marks the
  pointing model's slip, though nothing slipped; a smaller step is still chased. The
  hold avoids both (032's owner decision 25, reworded on 2026-11-18; its answer
  stands). FR-022 and FR-025 here are written for that answer. The probe of US5-S4
  observes that the mount does not move toward the new aim point and that the model's
  slip is not marked (plan.md § Prerequisites 5); since 038 only the second tells the
  hold from no hold, as in 032's US4-S8.
- Q: Could the run leave the mount moving after the observer's Stop? → A: Only in one
  window, which FR-020 closes. The run's re-centring turns Track off and on again. A
  Stop that lands between the two would be followed by the run's own start, and one
  that lands while Track is being asked makes Track answer `stopped`, which must not be
  read as a failure of the mount. The run therefore reads the stop generation before
  and after, whatever Track answered, and when it has changed it asks for Stop once
  more and ends as `stopped` (US4-S5 on the mount part alone, US5-S5 in a run).
- Q: What do 033, 035 and 036 offer, as drafted? → A: 033 a focus check that is an
  action of its own, `Imaging::focus_start` with `check` true, awaited with
  `Imaging::focus_ended`, with five verdicts (`good`, `slightly_soft`, `clearly_out`,
  `unmeasurable`, `no_record`) and sizes in arcseconds once a scale is known and in
  sensor pixels until then. 035 a `Stacker` whose `start` names its own folder,
  `runs/<UTC start>-<target>`, keeps every sub in that folder's `subs/` and lists it
  in `stack.json` by its path inside the folder, and whose view counts drops by
  reason; and one function,
  `crate::imaging::stack::service::label`, for the rule that turns a target's name into
  the last part of that folder's name, which the run's `run::label` calls rather than
  writing the rule again. 036 a `Steering` that says
  whether it is waiting for the tracker, takes the set point, and a tracker that says
  whether it is steady. plan.md § Interfaces names each call.
- Q: The focus check takes its own exposures. What is the first imaging frame for? →
  A: It is the run's first whole-sensor exposure, taken before anything is asked of the
  observer. It proves the camera, the scope link and the console's disk, so that a fault
  in any of them ends the run as a camera or save failure and is not mistaken for poor
  focus, and it puts a picture of the field on the Imaging page. The imaging solve then
  takes an imaging frame of its own, because by then the latest imaging frame is one of
  the focus check's (owner decision 20).
- Q: Where is the run's folder? → A: 035 names a stack's folder itself, so once the run
  stacks, its folder is the stack's and `run.json` lies beside `stack.json`. A run that
  ends before it stacks makes `runs/<run id>/` for its `run.json` at the ending (owner
  decision 23). The stack's subs are saved in `subs/` inside that folder, a folder per
  run, as the owner settled 035's owner decision 13 on 2026-11-17, and the plan
  document's shared storage says the same; the run's own imaging frames and the focus
  check's stay where 031 saves every imaging frame, under `frames/<UTC date>/`.
- Q: The imaging view of 031 holds nothing but its `state` while the camera is not
  `ready`. Where is a run that ended because the link was lost? → A: In the same view.
  `run` is filled in every state, as 035's `stack` is (FR-016, US3-S5).
- Q: Which endings are not in the plan document's list? → A: None now. Five were not
  when this trio was drafted, each a way a run can fail that the list then did not
  name, and the plan document's § 037-imaging-run now names each (owner decision 11):
  `mount_failed` (a GoTo that
  fails, as after five corrections), `target_changed` (the session's target was changed
  under the run), `not_centred`, `subs_failing` (ten subs in a row dropped, which is how
  "the solver keeps failing" shows while stacking, and also how cloud shows) and
  `stack_failed`. The list's "a failed save", "disk full" when this trio was drafted,
  is `save_failed`: the console cannot tell a
  full disk from another failed write in the words 031, 032 and 035 hand it, so the
  ending carries the system's own words (owner decision 7). A seventeenth reason,
  `pushed_away`, was added on 2026-11-16 with spec 038, which makes Track follow a push
  by hand instead of pulling the telescope back (owner decision 27). On 2026-11-19 the
  owner changed what reaches two of them: `subs_failing` is ten minutes of stacking in
  which a sub failed and none was stacked, no longer ten subs in a row (owner decision
  9), and `pushed_away` waits until a push has settled somewhere else or has lasted
  10 s (owner decision 27).
- Q: Which interfaces are not final? → A: Those of 030 to 036, which are drafts until
  they land, the lease of 031's US9 among them. The run reaches all of them only
  through the console's own five
  parts (FR-021, FR-029, FR-033 and the `steady` answer of FR-019), so the states, the
  routes, the page and the tools do not depend on their names. plan.md § Prerequisites
  2 and 3 list each name to confirm.

### Session 2026-11-15 (a second agent's review of this trio, and what it changed)

- Q: What did the review find, and what was done? → A: Twenty-two findings; every one
  was checked against the trio, `main` at 49bb210 and the drafts of 030 to 036, and
  all were applied. The ones that change what a builder writes:
  1. **The lease is asked before the camera part looks for an imaging frame in
     flight.** The focus aid and the stack expose almost without a gap, so a start
     while one of them holds the camera was answered `an exposure is in progress`
     instead of the holder's name. The camera part now looks for an imaging frame in
     flight inside `hold`, after the lease is taken (FR-021, US6-S7).
  2. **The script's stack never ends by a count.** `set_stack` sets the counts and the
     tracker's pause and leaves `running` alone; `end_stack` ends the stack. A test of
     an early story can no longer be broken by a rule a later story adds (FR-008).
  3. **Stop wins whatever Track answered.** A Stop that lands while a re-centre asks
     Track again makes Track answer `stopped`; the mount part compares the stop
     generation before it reads that answer (FR-020, US5-S5).
  4. **The mount-neutral rule is about code, not words.** The `too_low` message names
     the minimum altitude, as it must (SC-006).
  5. **A newly measured aim point under Track**: chased by the tracker as 032 was
     first drafted, and since 038 read as a push by hand when the step is more than 5′
     (the question above; 032's owner decision 25, which owner decision 26 cites).
  6. **Eleven stories, not eight** (twelve since the owner's split of 2026-11-19).
     The machine, the centring and the stacking stories
     were each likely to pass 64 KiB, so each is two: the run's states and then its
     failed steps and counts (US1, US2); the console's own mount, camera and solver
     parts and then centring (US4, US5); the limits and then the whole run on the
     console's own parts (US7, US8).
  7. **Whether a re-centre is asked depends on the aim point having moved**, read from
     the session before and after the solve, not on 032's outcome word: 032 leaves an
     aim point in force alone when the new median is within 10″ of it (FR-022,
     FR-024).
  8. **A dropped runner drops its task**, so a console dropped in the middle of a run
     still lets go of Track, as 015 requires (FR-007, US2-S9).
  9. **The time limit counts the clock**, pauses by the tracker included (FR-030,
     US7-S2).
  The rest tighten an oracle or a tolerance, or make a sentence say what the design
  does; each is in the scenario, FR or plan.md section it concerns.

### Session 2026-11-15 (an audit of the seven trios the series then had, side by side)

- Q: What did the audit find here, and what was done? → A: Four minor findings, each
  checked against this trio, the plan document and the drafts of 030 to 036, and all
  applied:
  1. **A1. The seam between 032 and 015 is 032's.** What the tracker does with a newly
     measured aim point is reachable from 032 on, with or without a run, so the owner
     settles it as 032's owner decision 25. Owner decision 26 here only cites it, and
     no longer says that its answer is written into 036. plan.md § The console's mount
     part and § Prerequisites 4 point there; the probe of US5-S4 stays.
  2. **A2. One rule for a folder's name.** `run::label` calls 035's
     `crate::imaging::stack::service::label` instead of writing 035's rule a second
     time; `run_id` is unchanged, and US1-S1 [now US12-S2] still pins `NGC 1976` to
     `NGC-1976`.
  3. **A3. The lease is 031's US9.** The names `Imaging::lease`, `Lease` and
     `Lease::take` are 031's own and no longer marked as assumed, 034 takes the same
     lease, and two scenarios now state the lease's refusal word for word:
     `the imaging camera is held by the imaging run` (US4-S4) and
     `the imaging camera is held by the focus aid` (US6-S7). The purpose `run` and the
     holder's name `the imaging run` are stable words of this spec in 031's records and
     refusals, so owner approval 2 names them.
  4. **A4. The run's own imaging frames are not capped.** The question below.
- Q: Are the run's own imaging frames capped as a stack's subs are? → A: No; this is a
  known limit. 036 caps the exposure of a stack's subs only: its steering decides the
  capped exposure inside 035's loop, before the stack's first sub, and sets it through
  the stack's own lease, putting the setting back as the stack ends. The run's first
  imaging frame and every imaging frame it solves are taken at the imaging camera's
  exposure setting as the observer left it, whatever the tracker's limit; 033's focus
  check exposes as 033 decides. On an alt-az mount the field turns, during a long
  exposure, about the point Track holds still, the target. Once the target is centred,
  the corners of such an imaging frame smear and its centre does not, the smear falling
  linearly toward the centre (`docs/research/2026-11-15-imaging-camera.md` § 7. Field
  rotation, sub length and the drive); before the first re-centre the centre smears in
  proportion to the target's distance from it. The solve's centre, which is all the run
  compares with the target, is not moved by the smear; stars lost in the corners can
  only leave a solve with fewer stars, and a solve that then fails is counted as any
  other (US2-S2). Capping these imaging frames too would need 036's steering to answer
  the capped exposure outside a stack, a getter 036 does not offer, and this trio may
  not add it. So the README says so and the field checklist has the observer look at
  the corners of the solved imaging frames (FR-040, FR-041; plan.md § Decisions and the
  alternatives rejected).
- Q: Did the audit move any citation? → A: No line that this trio cites had moved:
  `main` was still 49bb210, the commit the trio was drafted against, and every
  `path:line` in spec.md and plan.md was read again there. One citation was made more
  exact: the session target's type is cited at its `struct` line,
  `crates/finder/src/targets.rs:39`, in plan.md § The console's mount part.

### Session 2026-11-17 (the owner's answers)

- Q: What did the owner answer for owner decision 26, and what changed here? → A: Hold
  it: 032 holds a newly measured aim point back while Track is on and uses it once
  Track is off (032's owner decision 25, FR-036). The centring step turns Track off,
  takes the new aim point, re-centres and turns Track on again. FR-022's test of
  whether a solve moved the aim point now also counts an aim point 032 holds back;
  FR-024 asks for a re-centre on that test; FR-025's wait comes after the re-centre's
  Track off instead of before it. plan.md § The console's mount part (`repoint`'s
  steps, its wait now step 3), § The console's camera and solver parts (`aim_moved`),
  § What 030 to 036 offer, § Decisions, § Prerequisites 2, 4 and 5, § Risks, the
  README's field checklist step 8, the Refinement Record, tasks.md T018 and T023, and
  this spec's Clarifications, Edge Cases and Assumptions say the same. No story id,
  scenario id or FR number moved, and no scenario's text changed.
- Q: Where are a stack's subs, after 035's owner decision 13? → A: In `subs/` inside the
  stack's folder, which is the run's folder once the run stacks (035's owner decision 13,
  settled the same day as a folder per run). `stack.json` names each sub by its path
  there, and `stop(keep_subs)` removes them there. US2-S4's scripted path and US8-S1 and
  US8-S2 say the same.

### Session 2026-11-19 (round 1 probes and the reanalysis)

Probe round 1 built all eleven stories from this trio on 2026-11-19, tests first and
with both gates, in four agents, each starting where the one before stopped: US1 to US3,
US4 to US6, US7 and US8, and US9 to US11. The base held 030 to 034 and 038 as landed,
035 as a probe of its flipped trio built it and 036 as a probe of its folded draft did.
Every story built and passed its gates under 64 KiB, US1 at 93.5 % of it, two of them
only with a marked deviation (D13 and D14), and each wrote only its declared files. The
round found 3 blocking and 35 clarity defects, D1 to D38.
Six reviews then read the round, the trio and the code again and raised 71 more points
(A1 to A14, B1 to B9, C1 to C19, D101 to D110, E1 to E3, F1 to F16). One of them ran
round 1's 32 proposed cases with all eleven stories built: 31 passed on three runs, and
the other needed a behaviour nobody had built. All of it was folded into the three
files on 2026-11-19 (plan.md § Prerequisites 5 says what the round settled, and
§ Refinement Record, "What probe round 1 and the reanalysis changed", what changed).
Where two fixes touched one sentence they were merged. Nothing the fold added has been
run.

- Q: What did the owner answer before the fold? → A: "Take all 17", on 2026-11-19: each
  of the seventeen choices the reanalysis put to him takes its recommended answer.
  1. **A solve that measures no new aim point is not a centring try**, and four in a
     row end the wait (FR-024; owner decision 3).
  2. **The wait before the first imaging frame.** The mount part reads steady only when
     036's tracker was steady for the whole of 032's fit reach, 10 s or the exposure
     when that is longer. The fixed wait of 10 s that round 1 built is not folded
     (FR-019, US4-S1, US5-S6). A small follow-up to 032 will let the wait go.
  3. **A target plainly in the frame is not thrown away.** Within a quarter of the
     imaging frame's shorter side of the centre it is stacked with a warning. The fold
     applies this to both ways centring can give up: after three tries, and after four
     solves in a row that measured no new aim point (FR-024; owner decision 4).
  4. **A bump does not end the run.** It ends `pushed_away` only once 038's push has
     settled somewhere else or has lasted 10 s (FR-023; owner decision 27).
  5. **A short drop of the camera's link.** A failed take is asked again 5 s later, and
     the mount's ending is judged before the stack's failure (FR-009, FR-032; owner
     decision 9).
  6. **Dropped subs end the run by time.** It ends `subs_failing` after 600 s of
     stacking with a failed sub and none stacked; `behind` and `unprocessed` are not
     failed subs; and the 600 s are a constant, with no request key and no page field
     (FR-031; owner decision 9).
  7. **A focus check stopped from 033's own route** ends the run `stopped` (FR-029).
  8. **A target changed during a re-centre** ends the run `target_changed` with Track
     left off (FR-023; owner decision 12).
  9. **Subs are removed only after a saved stack of at least one sub** (FR-033; owner
     decision 22).
  10. **A console that exits mid-run.** The spec and the README say that the stacked
      picture is lost; `run.json` is refreshed at most every 60 s; and after a restart
      the page says that the last imaging run did not finish and where its subs are
      (FR-016, FR-017, FR-035). Saving the stack as it goes is asked of 035.
  11. **A full disk.** The stack part holds room back for the ending's own save, first
      answered as 64 MiB and sized to the stacked picture later that day (the last
      question of this session), and
      the start form shows the estimated size of the subs (FR-033, FR-035; owner
      decision 7).
  12. **The ended line** says that the stack was not saved when it was not, a count
      limit's line is the short one, and the counts are read again after the stack
      stops (FR-004, FR-012, FR-035).
  13. **A sentence for each cause of `stopped`, and a line that Track is still on.**
      The fold writes four sentences, one more than the answer's three, because
      answer 7 adds a cause. A stop of the run while it still slews also stops the
      mount (FR-005, FR-006; owner decisions 1 and 14).
  14. **`Stop imaging run` asks for a second tap** within 5 s (FR-035).
  15. **The focus pause.** A check during which the tracker was not steady is repeated
      once, the time the pause has left is shown, and a continue that comes too late
      is refused (FR-026 to FR-028).
  16. **US1's size.** US1-S3 to US1-S6 are now US2-S6 to US2-S9, tests only. The other
      answers added code to US1, so the fold moved the start's refusals of US1-S2 as
      well, as US2-S10. US1 was then estimated at about 56 KB, 86 % of 64 KiB, and the
      owner split it the same day (the last question of this session).
  17. **A second probe round** builds the changed stories before the flip.
- Q: What was folded without a choice? → A: Every other point, with the fix text its
  finding gave. The ones that change what a builder writes:
  1. **Another client's stack is left alone.** A run that finds a stack already running
     does not adopt it, stop it or remove its subs (FR-033).
  2. **A take answered with no frame** is a failed take (FR-021).
  3. **Three rules no scenario asserted now have one each**: an imaging frame in flight
     at a start (US4-S6), a record that cannot be saved at the ending (US3-S10) and the
     mount-neutral rule, read off the four source files (US8-S9). The two word lists
     are pinned too (US2-S11, since the split the first bullet of US12-S1).
  4. **Each second half of a scenario has its own number**, at the end of its story,
     so that it is graded by itself: US3-S10, US6-S10, US6-S11, US7-S7, US7-S8, US7-S9,
     US8-S8, US9-S13 and US10-S8.
  5. **`run.json` does not grow with every pause by the tracker.** A change between
     `stacking` and `paused_by_tracker` is listed and written at most once every 60 s
     (FR-017, US3-S11).
  6. **Words.** The run says `Saving the stack` while an ending saves it (FR-011). The
     sentences an observer could not act on are reworded, a time limit is said in
     minutes and an offset of 600″ or more in arcminutes (plan.md § The run's words).
     The MCP tools' descriptions say what an agent needs to run a night, and the view
     gains `limit_at` (FR-004). The README's table of endings gains what to do next,
     and its field checklist has eighteen steps, with Track left alone for 30 minutes
     first (FR-040, FR-041).
- Q: Which ids moved? → A: US1-S3, US1-S4, US1-S5 and US1-S6 are US2-S6, US2-S7, US2-S8
  and US2-S9, and three bullets of US1-S2 are US2-S10. No other scenario id, no story
  id and no FR number moved: every other new case has a new number at the end of its
  story, or is a bullet of the scenario it belongs to. At the fold the Work Graph was
  as it had been, and the trio had 90 scenarios where the probed draft had 58.
- Q: What was still open after the fold? → A: Four things, two of which the owner
  settled the same day (the next question): whether US1, then the largest story, is
  split further, and the size of the reserve against 035's saved stack. Still open are
  the second probe round, for everything the fold and those two answers added, and the
  field checks, which the README's checklist carries.
- Q: What did the owner answer after the fold, the same day? → A: Two more answers, on
  2026-11-19, and two corrections that needed none.
  1. **"Yes, split US1."** After every move of tests that could be made, US1 was
     estimated at about 56 KB, 86 % of 64 KiB, about 50 KB of it code, and factory
     builds have run to 1.35 times their probe build. So the story is two. **User Story
     12**, built first, writes what a run is handed and what it can say before any run
     exists: the five parts' traits with the types they exchange, the request and its
     settings, the constants and the two word lists, every ending with its reason and
     its sentence, `label` and `run_id`, and the scripted parts whole. Its four
     scenarios make no `Runner`. **User Story 1** keeps the machine and, with it, what
     only a run can show: the view's types, each state's message, the focus sentences
     and the warnings. The new story has the number 12 so that no story, scenario or FR
     number moves; it brings one new requirement, FR-042, and one new edge, US1 after
     US12, every other edge of the Work Graph as it was. Two clauses that needed no
     machine moved with it: US2-S11, the two word lists, is the first bullet of US12-S1,
     and the last bullet of US1-S1, `label` and `run_id`, is the first of US12-S2;
     US1-S1 and US1-S2 gained a clause each on the view's JSON. US2 has ten scenarios
     again and the trio 93. Estimated from round 1's measured bytes, and built by no
     probe: US12 about 43 KB, 66 %, and US1 about 30 KB, 46 % (plan.md § Risks,
     § Decisions and the alternatives rejected).
  2. **"Yes, size it."** The stack part's reserve is no longer a fixed 64 MiB, which
     could not hold the stack's own picture at the worked camera's full size. It is as
     large as the most 035 writes when a stack ends, for the camera's frame: 15 bytes a
     pixel of a sub and 16 MiB, about 260 MB at that size, written 4 MiB at a time so
     that nothing else on the console waits for it (FR-033, US8-S7; owner decision 7).
  3. **A pause that said something untrue.** A focus check that did not finish paused
     the run with the words of unmeasurable, "too few stars" and "check for cloud or
     dew", whatever had stopped it. Those words are now kept for a check that found no
     stars to measure; any other check that did not finish says so, with its reason, and
     says to check the imaging camera (FR-026, FR-029, US6-S10).
  4. **What 036's own fold changed underneath.** 036's reanalysis was folded the same
     day, and four things this trio said of 036 were corrected for it, with no choice
     put to the owner; the two new sentences are the fold's wording. First, 036 steers
     no stack whose first tracking error is over 180″ (036 FR-014), and inside that a
     step is at most 30″ a gap, and far less against an axis's motion (036 FR-008). So
     each of the two warnings for a target stacked off the centre has two forms, by the
     offset: up to 180″ it ends `; the stack's steering will bring it in.`, and over it
     `; it is stacked where it is.` (FR-024, US5-S3; the note under owner decision 4).
     Second, 036's tracker settles for 2 s after its first rates, a take-up and the
     keyhole pass, so the wait before an imaging frame of the run's own is 2 s to 3 s
     longer after each GoTo and each re-centre; FR-019's look-back is as it was (036
     FR-021; plan.md § The console's mount part). Third, 036's tracker is not steady
     while 038's push reads `moving`: the stack waits before its next exposure and the
     sub in flight is dropped as `not_steady`, which at the fold was not known (036
     FR-021; plan.md § Risks). Fourth, 036's stack also waits, with the tracker steady,
     while its exposure limit is under half the exposure held (036 FR-022). The run's
     sentence for `paused_by_tracker` said `Waiting for the mount to settle`, which is
     not true of that wait, and now says `Exposures paused`, naming no part; 036's own block on
     the Imaging page says why (FR-033, US2-S3; plan.md § The run's words). A sentence
     of the run's own for each cause is not folded: it needs the stack part to carry
     036's word. No story, scenario or FR number moved, and no scenario was added.

## Owner decision 26, settled

26. **The tracker and a newly measured aim point** is 032's owner decision 25, cited
    here and not decided here. **Settled by the owner on 2026-11-17: hold it.** 032
    holds a newly measured aim point back while Track is on and uses it once Track is
    off (032's FR-036, US4-S8). A run's centring step turns Track off, takes the new
    aim point at the first live pointing after it, re-centres and turns Track on again:
    FR-022's test of whether a solve moved the aim point counts an aim point 032 holds
    back, and FR-025's wait comes after the re-centre's Track off. The hazard is
    reachable from 032 on, whenever an imaging solve measures a new aim point while
    Track is on, with or without a run, which is why 032's trio holds the answer; this
    spec changes neither 032 nor 015. Instead: the aim point used at once, as 032 was
    first drafted, under which 015's tracker closed the whole step from the old aim
    point at a rate nothing bounds, under Track's guards and outside GoTo's, for up to
    the 5 s a re-centre then waited with Track on (Clarifications), and under which,
    since 038, a step of more than 5′ is read as a push by hand that marks the pointing
    model's slip instead; or one of 032's
    other answers (a bound on the tracker's rate in 036, Track restarting through GoTo,
    or the chase accepted and bounded by a probe), none of which changed a line here.
    The probe of US5-S4 observes that the simulator's mount does not move toward the
    new aim point in those seconds and that the model's slip is not marked (plan.md
    § Prerequisites 5).

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| `run.rs` with three child modules | One file, as the plan document first named it (it now names the three children too) | Spec owner decision 25: the machine in `run.rs` then imports nothing of the session, the mount or the camera, which is what keeps it mount-neutral and testable on the script alone, and each story's diff stays small. |
| Five small traits and a struct of them | One `Steps` trait | US4, US6 and US8 each add real parts; a test of one story uses that real part with the script for the rest. One trait would make every story implement all of it. |
| The traits and the script are complete from US12, and the machine grows by story | The whole machine in US1; a trait or a script that grows | No scripted test implements a part itself, so later stories never touch an earlier test; a machine written whole would put the retries, the focus pause, the centring loop and the limits in one story with no real part to prove them against, well over 64 KiB. |
| A scripted stand-in in the library | A copy in each of nine test modules; a new support file | Spec owner decision 16. |
| The lease is two calls of the camera part, `hold` and `release` | A sixth part; a lock in the `Runner` | The plan document's amendment 1: one lease, 031's, and no lock of the run's own. The camera part is where the run meets 031. |
| The lease is taken before Track is asked | After the mount has arrived | A run that cannot expose should not move the telescope. |
| The run holds the lease while it slews, exposes, solves and centres, and hands it over to the focus check and the stack | The run holding it for its whole length; the run taking none | Spec owner decision 15: 033's `focus_start` and 035's `Stacker::start` take the lease themselves, under their own names, so a run that kept it would refuse its own steps. Taking none would let a single imaging frame or a set of darks in while the run slews, solves or re-centres. |
| `camera.release()` is called at every ending whether or not the lease is held | Tracking in the machine whether it is held | One rule a test can read off the call list: the last call of every run gives the camera back. |
| A first imaging frame, then 033's own focus check, then an imaging frame for the solve | Measuring focus on the first imaging frame; no first imaging frame | Spec owner decision 20: the plan document's order, with 033's check as its draft gives it. `Imaging::solve` solves the latest imaging frame, which after a check is one of the check's. |
| The run's view rides in the imaging view, in every state | A top-level `run` in the session; only while `ready` | The plan document puts `run` in `imaging`. A run that ended because the link was lost must still be readable; 035's `stack` does the same. |
| The run's record lies in the stack's folder, and a run that never stacks makes its own | The run makes the folder and tells the stack to save there | 035's `Stacker::start` names its folder and takes none (spec owner decision 23). One folder then holds the stack and what the run did. |
| `run.json` rewritten whole at each change of state | JSON Lines of events | It is small, changes about ten times and after that at most twice a minute (§ The record), and one whole file is what the field note copies. |
| An ending asks nothing of the mount, but a stop of the run while it still slews, which asks for Stop | Stop at every ending; no Stop at any | Spec owner decision 1: Track's own guards keep watching, and the target stays in view. Before anything was exposed there is no target in view to keep, and a run that says it stopped while the tube still swings misleads (spec owner decision 14, as changed on 2026-11-19). |
| Four endings share the reason `stopped`, each with its own sentence | One sentence, `Stopped by the observer.` | The one sentence covered a run stopped with Track left on, a mount at rest, an MCP client's stop and a stack or a focus check stopped from its own route; the observer could not tell whether the telescope still moved. The reason word stays one, so `ENDINGS` and every client's switch on it are unchanged. |
| `repoint`, `stack.start`, `stack.stop`, the aid's open and close and the Stop of a run stopped while it slews are awaited unguarded | Guarded like the other steps; the runner's stop signal handed to `repoint` | Dropping one half-way would leave Track off, or a stack or a focus aid running, with the run already ended. A stop of the run therefore waits for them: for a re-centre about 7 s (the 5 s of its wait, Track's own 2 s for the axes to rest, one Track off and one Track on), after which Track is on, as after any stop of the run that has exposed (spec FR-005, owner decision 14). Handing the signal in would add a parameter to a trait for a wait that short. |
| Durations are measured on tokio's clock, stamps on the console's | Both on the console's clock | The tracker does the same (`crates/finder/src/tracking.rs:4`); `FixedClock` in a scripted test never moves. |
| The run waits for a steady tracker without a time limit | A timeout | Rule 6 and rule 1: how long a tracker is unsteady is the mount's affair, a meridian flip among them; the observer can always stop. |
| Save failures are one ending | A free-space check | Spec owner decision 7: the words reach the run as text, and a new dependency is the owner's to approve. |
| `run::label` calls 035's `crate::imaging::stack::service::label` | A copy of 035's rule in `run.rs` | The run's id and the stack's folder must end alike; one function cannot drift from itself, and the machine still reads no session (spec § Clarifications, the audit). |
| The script's `set_stack` sets the counts and the tracker's pause; `end_stack` ends the stack | One call that replaces the whole `StackProgress` | A test of US2 or US3 that set counts with `running` at its default would be ended by US7's rule for a stack that no longer runs, in a file US7 may not edit. |
| A dropped `Runner` aborts its task | Leaving the task to the runtime; a weak reference to the state in each part | The console's parts hold clones of the console's state, the owner of Track among them. 015 lets Track go when the console is dropped (`crates/finder/tests/motion/tracking/routes.rs:456`); a run must not keep it. One `Drop` does that; weak references would put an upgrade in every call of every part. |
| Twelve stories: the machine, the centring and the stacking each in two, and the machine's parts, words and script ahead of it (the next row but one) | Eight stories, split after the probe if one passes 64 KiB | A split after dispatch is a changed spec and a relaunch (§ Plan traps 5). US1 and US2 are cut where every step answers against where one fails; US4 and US5 where the run points against where it points again; US7 and US8 where the script suffices against where the simulator is needed. |
| US1's tests of a refused start, a mount that is not steady, a stop, the mount's endings and a dropped runner are written by US2, in US2's test module | Leaving them in US1; a story of its own for the script | US1 was 93.5 % of 64 KiB as probed, and the owner's answers of 2026-11-19 added code to it. Tests can move with no change to the Work Graph, where a new story cannot (spec § Clarifications, the owner's answer 16; § Risks). The move left US1 at 86 %, and the owner then split it as well (the next row). |
| The run's parts, its words and its script are a story of their own, US12, built before the machine; the view's types and the words made from a run's state stay with the machine, in US1 | US1 as one story of about 56 KB; the script alone as a story; every word and every type in US12, the state's messages and the view among them | The owner's split of 2026-11-19: US1 was estimated at 86 % of 64 KiB with about 50 KB of code, and factory builds have run to 1.35 times their probe build. The cut is by what can be proven without a run. An ending's reason and sentence, the request, `run_id` and the script's contract need none, and US12 asserts them directly. A state's message, a focus sentence and a warning are made from a run's own state, and `RunView` is `#[non_exhaustive]`, so a test outside the crate cannot build one: with those in US12 it would hold code its own tests could not reach, or new public functions written only to be tested, and would be estimated at about 55 KB, 84 %, against 66 % as cut. The script alone would have left the machine with every type, at about 44 KB. The story has the number 12 so that no other id moves, as 035's US11 has. |

Added at the re-anchor of 2026-11-20, for principle V of constitution 1.3.0, which asks a
plan to name what it looked at and why it turned it down:

| Decision | Rejected | Why |
|---|---|---|
| The imaging run is this project's own code, a machine in the console over five parts | Ekos's scheduler and sequencer in KStars (GPL-2.0-or-later) | KStars is a desktop application that would replace the console as the place the run is driven, not a service the console can call. Nothing of it is copied. |

## Dated notes and probe asides moved out of the trio

Each passage below was in the trio as quoted, with the file it was in before the
split; the trio now holds the same text without it.

- From spec.md:

  > asserted by US2's test module (US2-S6 to US2-S10): those scenarios were this story's until 2026-11-19, and were moved, tests only, to keep its diff under 64 KiB. The same day the owner cut the parts' traits, the endings' words and the script out of it, as User Story 12 (plan.md § Risks).

- From spec.md:

  > Scenarios US2-S6 to US2-S10 assert the machine as US1 leaves it. They were US1's until 2026-11-19 (US2-S6 to US2-S9 were US1-S3 to US1-S6, and US2-S10 three bullets of US1-S2). Their tests pass before this story's code is written and must still pass after it.

- From spec.md:

  > This story is built first, before User Story 1 (Work Graph). It has the number 12 because the owner cut it from User Story 1 on 2026-11-19, when that story was estimated at 86 % of 64 KiB with every test moved out of it that could move, and numbering it so left every other story, scenario and requirement with its number. It writes every public

- From spec.md:

  > (FR-005, US2-S7, US4-S4; owner decision 14 as changed on 2026-11-19).

- From spec.md:

  > A known limit (Clarifications), which the field checklist has the observer look for (FR-041).

- From spec.md:

  > words gives it; and `label` and `run_id` (US12-S1, US12-S2; added on 2026-11-19 with User Story 12).

- From spec.md:

  > imaging frames too, which needs a getter 036 does not offer (Clarifications);

- From plan.md:

  > (spec owner decision 14, as changed on 2026-11-19).

- From plan.md:

  > US2-S6 to US2-S10, which were US1's scenarios until 2026-11-19 and assert the machine as US1 leaves it: their tests pass before US2's code is written and after it.

- From tasks.md:

  > Built first, before User Story 1 (spec.md § Work Graph). It has the number 12 because the owner cut it from User Story 1 on 2026-11-19, and its tasks are numbered after the last of the others for the same reason. It writes no `Runner`, no view and no message of a state.

## Assumptions as they read before the split

- **Earlier features are in place.** 014, 015, 016 and 017 have landed, and so, by
  2026-11-19, have 030 to 034 and 038; 035 is being built and 036 is a draft (plan.md
  § Prerequisites 1). Every `path:line` here was read at 49bb210, and read
  again there after the audit. A file that 030 to 036 create is named without a line,
  with the spec that creates it.
- **Outside the factory:** rebuilding the console; the field checks at the telescope,
  which follow the checklist of FR-041: the whole run on a bright target, started once
  from the page and once by an MCP client; how many centring tries a real night needs
  and how far off the first solve finds the target; whether 90″ suits the real mount;
  the real focus pause, with the focuser turned by hand; a run through a Stop; that the
  real mount does not move toward a newly measured aim point between a first imaging
  solve and the re-centre's Track off (032's owner decision 25); whether the corner stars of the run's own imaging frames trail at
  the exposure setting used for subs (Clarifications); and a design review of the run
  block on a phone in the dark. GoTo and
  Track must be proven on the real mount before any of it (the plan document, § Before
  any flip, 4).

## Risks as they read before the split

Each row below was rewritten for the split to drop what probe round 1 had or had
not run, or, for the newly measured aim point under Track, which 032 has closed,
left out; plan.md § Risks holds what is in force.

| Risk | Mitigation |
|---|---|
| A story's diff passes 64 KiB. | Measured in probe round 1 (§ Prerequisites 5): all eleven were under, US1 at 61,307 B, 93.5 %, and no other above 62 %. The fold of 2026-11-19 added code and cases to every story, and the owner then split US1, so what follows is estimated from the round's measured bytes, each within about 1 KB but for the two halves of the split, which are within about 2 KB: US12 43.4 KB (66 %), US1 30.0 KB (46 %), US2 35.2 KB (54 %), US3 47.1 KB (72 %), US4 40.9 KB (62 %), US5 29.1 KB (44 %), US6 37.4 KB (57 %), US7 14.8 KB (23 %), US8 28.2 KB (43 %), US9 37.7 KB (58 %), US10 23.9 KB (36 %) and US11 20.3 KB (31 %). **No story is estimated above 72 %.** Before the split US1 was the one story above 85 %, at about 56.4 KB, even after US1-S3 to US1-S6 and the start's refusals of US1-S2, about 12.5 KB of tests, had become US2-S6 to US2-S10 (spec.md § Clarifications, the owner's answer 16, widened by the fold): what was left was about 50 KB of code and two scenarios. The split (spec.md § Clarifications, the last question of the session of 2026-11-19) gives US12 the parts' traits, the endings' words and the request, about 15.8 KB as round 1 built them with what the fold added, the script, about 12.4 KB, and four new scenarios that make no run, about 14.4 KB of tests; and leaves US1 the view and the machine, about 23.4 KB, and its two scenarios, about 6.2 KB. The two halves come to about 17 KB more than US1 did, all of it US12's new tests. A factory build has run to 1.35 times its probe build: at that US12 would be about 89 % and US3 about 97 %, so US3 is the next story to cut, and after it US4, whose scenario of the guards (US4-S4) could become a story of its own. The second probe round measures all of it; US12 and US1 as they now are have been built by none. |
| US12 writes public items that nothing in the crate calls until US1 lands: the traits, `Ending::message`, the script. | They are public items of a library, which the lints do not count as dead code, and US12's test module calls each of them. Not built: the second probe round builds US12 by itself, with both gates. |
| A console dropped in the middle of a run keeps Track on through something the imaging state of 030 to 036 holds. | Probed in 037's round 1. A console dropped while a run of its own five parts stacks has its mount at rest within 0.05 s, which stays at rest. Its session is let go 2.95 s after the drop, exactly as when a stack started by hand runs with no run. 035's stack holds it until its take ends. 037 adds no delay of its own. A test that drops a console while a stack runs waits more than 015's 2 s for the session. US2-S9 proves that the runner lets its parts go. Unprobed: a console dropped during a focus pause, which leaves 033's aid, opened by the run, with no owner. In the binary the state lives until exit, so only tests are touched. |

## The Refinement Record's prose

As it stood before the split; the record's yaml is in each piece's plan.md.

Drafted on 2026-11-15 by the grilling session against `main` at 49bb210, from
`docs/plans/2026-11-15-specs-030-037-imaging.md` with its § Mount-neutral by design and
its amendments, the reviewed trios of 030, 031, 032 and 034 and the drafts of 033, 035 and
036 in the same worktree, a read of `crates/finder/src/goto.rs`, `tracking.rs`,
`session.rs`, `api.rs` and the `mcp` module, the specs 014 to 017 and 025, and the
research on prior art with its verification. A first draft of this trio, written at
dbc2b5b before 033, 035 and 036 had drafts, was read whole and replaced.

A second agent then reviewed the trio against the plan document, the code at 49bb210
and the drafts of 030 to 036, and returned twenty-two findings: one blocker, six majors
and fifteen minors. Each was checked again against the same sources and all were
applied; spec.md's last session under Clarifications lists what a builder writes
differently because of them. The largest change is the Work Graph: eleven stories for
eight, with the machine, the centring and the stacking each cut in two, so every story
id, scenario id and FR number of the reviewed draft above US1-S5 and FR-006 has moved.

An audit then compared the seven trios the series then had, side by side, and gave this
one four
minor findings, all applied (spec.md § Clarifications, the last session): owner
decision 26 now cites 032's owner decision 25 (A1); `run::label` calls 035's
`crate::imaging::stack::service::label` (A2); the lease is 031's US9, its names no
longer marked as assumed, US4-S4 and US6-S7 state its refusal word for word, and owner
approval 2 names the purpose `run` and the holder's name `the imaging run` (A3); and the
run's own imaging frames, which 036 does not cap, are a known limit that the README and
the field checklist's step 8 state, with US11-S2 reading it (A4). No story id, scenario
id or FR number moved. Every `path:line` was read again at 49bb210, where `main` still
was: none had moved, and one was made exact (§ Prerequisites 7).

On 2026-11-17 the owner settled 032's owner decision 25, which owner decision 26 cites,
with the hold: FR-022, FR-024 and FR-025, `repoint`'s steps (its wait is now step 3,
after Track off) and `aim_moved` were rewritten for it (spec.md § Clarifications, the
last session). No story id, scenario id or FR number moved, and no scenario's text
changed.

**Until probe round 1 no probe build had been run.** No story of this trio had been
compiled, linted or tested, no interface above had been built, and no diff had been
measured: the sessions before it wrote documents only. Every `path:line` was read at
49bb210. Every number in a scenario was either stated by the scenario as its own oracle
or worked by hand: 1/20 of the
simulated imaging frame's 822 pixels at 2.1845″ a pixel is 89.8″, under the 90″ floor;
the centre (728, 544) of the simulated finder camera's frame is 127 pixels from the true
aim point (700, 420), about 3800″ at 29.78″ a pixel.

**What probe round 1 and the reanalysis changed** (2026-11-19; spec.md
§ Clarifications, the session of round 1 probes and the reanalysis). Four agents built
the eleven stories from this trio, in order, tests first and with both gates, and found
3 blocking and 35 clarity defects, D1 to D38 (§ Prerequisites 5 says what the round
settled). Six reviews of the round, the trio and the code then raised 71 more points
(A1 to A14, B1 to B9, C1 to C19, D101 to D110, E1 to E3, F1 to F16), and the owner
answered their seventeen choices with "Take all 17". All of it was folded on
2026-11-19, in three passes. The changes that reach across stories:

- **The three blocking defects.** US4-S5 reads rest from 2 s after a Stop (D13). The
  mount part reads steady only when 036's tracker was steady for the whole of 032's
  fit reach, by `steady_between` and not by round 1's fixed wait (D14 and the owner's
  answer 2: FR-019, US4-S1, US5-S6). US10's continue is read back with
  `get_imaging_run` after a settle (D30, now US10-S8).
- **What ends a run.** A solve that measured no new aim point is no try, and a target
  near the centre is stacked with a warning whichever way centring gives up (FR-024);
  a push ends the run only once it has settled or lasted 10 s, and a target changed
  during a re-centre leaves Track off (FR-023); a failed take is asked again 5 s
  later, and the mount's ending is judged before the stack's failure (FR-009, FR-032);
  subs that keep failing end the run by time, not by a count (FR-031); and a focus
  check stopped from 033's own route ends the run (FR-029).
- **What a run keeps, and says.** Subs are removed only after a saved stack of at least
  one sub, another client's stack is left alone, and room for the stacked picture's
  files is held back for the ending's save (FR-033); `run.json` is refreshed while the
  count rises, lists the
  tracker's pauses at most once a minute, and an unfinished run is said after a
  restart (FR-016, FR-017); four endings share the reason `stopped`, each with its
  sentence, an ending says whether the stack was saved and whether Track is still on,
  and a stop of the run while it still slews stops the mount (FR-004 to FR-006); the
  sentences an observer could not act on are reworded (§ The run's words).
- **The focus pause** repeats a check during which the tracker was not steady, shows
  its deadline, and acts on the first resume only (FR-026 to FR-028).
- **The interfaces of US1** (US12's since the split). `MountSteps` gained `stop`,
  `track_on` and
  `steady_between`, `Centre` `near_arcsec`, and the script `unsteady`,
  `Step::StackStop` and `Step::CloseAid`; the view gained `stacking_since`,
  `limit_at`, `pause_ends_at`, `ended.saved` and `ended.track_on`, and the imaging
  view `unfinished_run`.
- **The page, the tools and the README.** The block asks for a second tap before it
  stops a run, disables its buttons while a request is out, and shows the size of the
  subs, the time stacked against the limit and the time a pause has left (FR-035);
  the tools' descriptions say what an agent needs to run a night (§ The MCP tools);
  the README's table of endings says what to do next, and its field checklist has
  eighteen steps (§ The README).
- **Tests moved, with no code.** US1-S3 to US1-S6 are US2-S6 to US2-S9, and the start's
  refusals of US1-S2 are US2-S10, so that US1's diff stays under 64 KiB (the owner's
  answer 16, widened by the fold when the other answers added code to US1; § Risks).
- **Scenarios.** The trio had 90 at the fold where the probed draft had 58, and has
  93 since the split of US1. Each second half of a
  scenario has its own number (US3-S10, US6-S10, US6-S11, US7-S7 to US7-S9, US8-S8,
  US9-S13, US10-S8); three rules that no scenario asserted have one each (US3-S10,
  US4-S6, US8-S9), and so have the two word lists (US2-S11, since the split a bullet of
  US12-S1); and every other new case
  is a bullet of the scenario it belongs to or a new scenario at the end of its
  story, so no scenario id moved but US1-S3 to US1-S6.

The records below change with them: every case added has a clause in its scenario's
`assertion`; the moved scenarios are listed under US2, in US2's test module; and US11's
shared write of the README names 036's US8, which writes it after US7 in 036's folded
record. Two strings the record repeats were shortened, with no change of meaning, to
keep it under the 64 KiB a record may be. No story was over 64 KiB as built. On the
folded figures US1 was estimated at about 56 KB, 86 % (§ Risks). None of what the fold
added has been run: the second probe round builds the changed stories before the flip.

**The split of US1** (2026-11-19, after the fold; spec.md § Clarifications, the last
question of that session). The owner answered "Yes, split US1", and the record gained
US12, which is built first. It holds four scenarios: the two word lists, which was
US2-S11, with every ending's reason and sentence; `label` and `run_id`, which was a
bullet of US1-S1, with the request and the settings; and two for the script. US1 lost
its writes of `crates/finder/src/imaging/mod.rs` and `script.rs`, and gained a line for
US12's scenarios, which it must leave passing. US12 and US3 both write
`crates/finder/src/imaging/mod.rs`, and US3 follows US12 through US1; the record has no
shared write for the two, because the validator takes a `merge_order` entry only between
two stories joined by an edge of their own, and the Work Graph gained no edge but US1
after US12. The same day the stack part's reserve was sized to the stacked picture
(US8-S7) and the pause after a focus check that did not finish got words of its own
(US6-S10). To make room for US12's block the record was re-indented, and nothing else:
its list items stand under their keys, and every level under `stories` is one space
deep. The record parses to the same object before and after each of the two steps, no
`checks` list was narrowed and no clause of an assertion dropped. Nothing of the split
has been built.

**036's second fold** (2026-11-19, after the split; spec.md § Clarifications, the fourth
point of that session's last question). 036's fold of its own reanalysis changed four
things this trio said of it, and they are corrected here with no choice put to the
owner; the two new sentences are the fold's wording. Its steering stands back from a
stack that begins more than 180″ from the set point, so each of the two warnings for a
target stacked off the centre has a second form, `; it is stacked where it is.`, chosen
by 036's own `FAR_ARCSEC` (FR-024; US5-S3, whose two examples at 400″ now read the
second form, and which gained a bullet at 180″ and 150″ for the first). Its tracker
settles for 2 s after its first rates, so each wait for a steady tracker is that much
longer (§ The console's mount part). It is not steady while 038's push reads `moving`,
which § Risks had as not known. And its stack also waits for its exposure limit to rise,
with the tracker steady, so the sentence of `paused_by_tracker` names no
cause (US2-S3). No story, scenario or FR number moved and no scenario was added. In the
record below US5-S3's assertion gained one clause, and nothing else changed: it is
65,440 B of the 65,536 a record may be. None of it has been run.

§ Prerequisites 2 and 3, the re-anchoring, are still to do. Until then the record
below names what the drafts named: its
tests are the scenarios, its writes are the files § Project Structure names, and its
shared writes name the merge edge that orders two stories of this spec on one file, or
the story of 030 to 036 that last writes the file among the drafts as they stood on
2026-11-15, read from each draft's own record. Each of those names is checked again at
the re-anchoring. A test file an earlier story of this trio creates does not exist yet
and cannot be listed as an existing test; what a later story must leave passing in it
is under that story's `preserved_behavior`. The record's `open_questions` are empty
because the validator refuses a record that holds any; what is not established is in
§ Risks and § Prerequisites, and what the owner must settle is in spec.md § Owner
decisions, with the seam between 032 and 015 settled as 032's owner decision 25, which
decision 26 cites. Nothing in this record is an owner
approval: those are in spec.md, and flipping to `ready` gives them.
