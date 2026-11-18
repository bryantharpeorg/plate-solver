---
state: draft
depends_on_landed: ["014-goto", "015-tracking", "016-mcp", "017-console-stop-confirm", "030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "034-dark-and-flat-frames", "035-live-stack", "036-tracking-from-subs", "038-track-follows-a-push"]
# Drafted 2026-11-15 against `main` at 49bb210 (the drafting worktree's HEAD,
# 49bb210fe4860dd98ba24727074e7bd3cc0d7124) from
# docs/plans/2026-11-15-specs-030-037-imaging.md, by the grilling session of that day.
# A first draft was written against dbc2b5b before 033, 035 and 036 had drafts and
# before the plan document gained "Mount-neutral by design" and its amendments; this
# text replaces it and is in line with both.
# Not yet probe-built: no story of this trio has been compiled or run (plan.md
# § Refinement Record). It builds on code that 030 to 036 create and that does not exist
# yet. Their interfaces are taken from their plan.md in the same worktree, name for
# name: the lease of the imaging camera, which the plan document's amendment 1 gives
# 031, from 031's US9, and the rule for a stack's folder name from 035. The trio is
# re-anchored to `main` after its seven predecessors land and before the flip.
# A second agent reviewed the trio the same day; its twenty-two findings are answered
# in this text, which has eleven stories where the reviewed draft had eight. An audit
# of the seven trios the series then had, side by side, then gave it four more (A1 to A4), also answered
# here (the last two sessions under Clarifications; plan.md § Refinement Record).
# Citations checked against 49bb210 (49bb210fe4860dd98ba24727074e7bd3cc0d7124): after
# the audit every `path:line` in spec.md and plan.md was read again at that commit.
# It stays draft until the owner settles the owner decisions and flips it to `ready`,
# which gives the approvals listed under "Owner approvals" and dispatches it.
---

# Feature Specification: The Imaging Run

**Feature Branch**: `037-imaging-run`

**Created**: 2026-11-15

**Status**: Draft

**Input**: Specs 030 to 036 give the console an imaging camera, imaging solves, a measured
aim point, a focus reading, calibration frames, a live stack and tracking from the subs,
each with its own route and MCP tool. This spec joins them into one procedure, the
imaging run: one ask, from the Imaging page or an MCP client, that goes to the session's
target, tracks it, takes a first imaging frame, checks focus, centres the target on the
imaging camera's sensor and stacks subs until it is stopped or a limit is reached, and
that ends safely whatever goes wrong. The run is a client of the console's GoTo, Track
and Stop and adds no other way to move the mount. An MCP client chooses the target and
the settings, starts, watches and stops; it never puts the steps in order itself.

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
  person (below). Scale comes from the imaging solve. The owner's telescope appears only
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
  by hand instead of pulling the telescope back (owner decision 27).
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
  6. **Eleven stories, not eight.** The machine, the centring and the stacking stories
     were each likely to pass 64 KiB, so each is two: the run's states and then its
     failed steps and counts (US1, US2); the console's own mount, camera and solver
     parts and then centring (US4, US5); the limits and then the whole run on the
     console's own parts (US7, US8).
  7. **Whether a re-centre is asked depends on the aim point having moved**, read from
     the session before and after the solve, not on 032's outcome word: 032 leaves an
     aim point in force alone when the new median is within 10″ of it (FR-022,
     FR-024).
  8. **A dropped runner drops its task**, so a console dropped in the middle of a run
     still lets go of Track, as 015 requires (FR-007, US1-S6).
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
     time; `run_id` is unchanged, and US1-S1 still pins `NGC 1976` to `NGC-1976`.
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

## Owner decisions

Each is a design or taste call the plan document leaves open, but for 11, 13, 15, 22
and 25, whose recommended answer the plan document now writes into its shared
interfaces; they stay here, under their numbers, for the owner to settle at the flip.
None of the first twenty-five is settled; the twenty-sixth is a decision of 032's, cited here because a
run crosses it, and settled by the owner on 2026-11-17; the twenty-seventh, added on 2026-11-16, is unsettled too. A different answer is written into the trio and re-validated before the
flip, never left to a story.

1. **What an ending does to the mount.** Recommended: nothing. An ending that a guard or
   Stop caused finds the mount as that guard left it, at rest; every other ending (the
   run's own stop, a limit, a camera, solver, save or focus ending) leaves Track on, so
   the target stays in view and Track's own guards still watch it. Instead: every ending
   asks for Stop.
2. **When the target counts as centred.** Recommended: within the larger of 1/20 of the
   imaging frame's shorter side on the sky and 90″ of the sensor's centre: 90″ for the
   worked telescope, whose imaging frame is 0.4997° high. The floor is one and a half times
   GoTo's own 60″, below which a re-centre cannot reliably land. Instead: a fixed 2′; or
   1/10 of the shorter side.
3. **How many centring tries.** Recommended: three re-centres, so at most four solves.
   Instead: five, as GoTo's own corrections.
4. **A target that cannot be centred.** Recommended: the run ends `not_centred`.
   Instead: stack anyway with a warning whenever the target is inside the imaging frame.
5. **Going on from a focus pause.** Recommended: one action with a switch. "Check focus
   again" repeats the focus check, pausing again if it is still clearly out; "Go anyway"
   goes on unchecked. Instead: one action that always goes on unchecked; or the run
   resuming by itself when the focus aid's reading turns good.
6. **How long a focus pause may last.** Recommended: 15 minutes, then the run ends
   `focus_timeout` with Track still on. Instead: no limit; or 5 minutes.
7. **A full disk.** Recommended: one ending, `save_failed`, when any save fails, with the
   system's words. Instead: a free-space check before each sub, which needs a new
   dependency such as `fs4` (MIT OR Apache-2.0) and its own approval.
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
10. **The target.** Recommended: the run takes the observing session's target. An MCP
    client sets it with the existing `set_target` and then starts. Instead: a `target`
    argument on the route and the tool.
11. **Five endings the plan document's list first lacked, and now names**
    (Clarifications). Recommended: all
    five, each with its own reason word. Instead: fold `mount_failed` and `not_centred`
    into one, and `subs_failing` into `solver_failing`.
12. **A changed target.** Recommended: when the session's target is changed under a run,
    the run ends `target_changed` and leaves Track as it is. Instead: refuse a target
    edit while a run is on.
13. **Where the run block sits on the Imaging page.** Recommended: first, above the
    Camera section, because Start is the page's main action. Instead: last.
14. **Stopping the run does not stop the mount.** Recommended: `Stop imaging run` and
    `stop_imaging_run` end the run and save the stack; the top bar's Stop and the MCP
    `stop` stop the mount and end the run too. A stop of the run that arrives while it
    re-centres is answered once Track has been asked again, within about 5 s, and
    leaves Track on. Instead: the run's stop also asks for Stop.
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
17. **No new Playwright check.** Recommended: none. Vitest drives the run block through
    031's `useImaging`, and US8's `cli` test starts the built binary. The block holds no
    picture and uses only existing classes. Instead: an `imaging-run.spec.ts` in the
    simulate project, which means an approved edit of `playwright.config.ts`.
18. **What `run.json` holds.** Recommended: the target, times, settings, states, counts
    and the ending; never the site, the node's address or its token. Instead: the site
    too, for later analysis.
19. **Which run tools are logged and rate-limited.** Recommended: `start_imaging_run`
    only, the one that moves the telescope. Instead: all three that change something.
20. **The first imaging frame, and 033's focus check after it.** The plan document's
    order is a first imaging frame, then the focus check; 033's focus check takes its
    own exposures. Recommended: keep the order. The run takes one whole-sensor imaging
    frame first (Clarifications), then runs 033's check, and the imaging solve takes an
    imaging frame of its own: one exposure more than the least possible. Instead: drop
    the first imaging frame and let the focus check's exposures be the run's first,
    which changes the plan document's order; or have 033 offer a verdict for a saved
    imaging frame, so that one imaging frame is taken, measured and solved.
21. **No focus record yet.** Recommended: the verdict `no_record` pauses the run and
    opens the focus aid, as clearly out does, with its own words; using the focus aid
    makes the record, and "Go anyway" goes on. Instead: warn and go on.
22. **Subs that are not kept.** Recommended: with `keep_subs` false the run removes each
    sub's file and its solve sidecar when the stack has ended, from the list in 035's
    `stack.json`, and removes nothing when the stack could not be saved; the disk still
    holds the subs while the run stacks. Instead: 035's stack removes each sub once it
    is stacked or dropped, an edit to 035.
23. **Where `run.json` lies.** Recommended: in the stack's own folder once the run
    stacks, and in `runs/<run id>/` when the run ends before that (Clarifications).
    Instead: 035's `Stacker::start` takes a folder from its caller, and the run makes
    the folder at its start.
24. **A region of interest.** Recommended: a run is refused while the imaging camera's
    settings hold a region of interest, with words that say to clear it: 032 takes an
    aim reading from whole-sensor imaging frames only, and the centre of a region is not
    the set point. Instead: measure centring through the solve's own map at the sensor's
    centre and let such a run go on without aim readings.
25. **Three child modules** (once a departure from the plan document, which first named
    one file, `run.rs`, and now names these three children of it). Recommended:
    `run.rs` holds the machine, `run/script.rs` the scripted
    stand-in, `run/store.rs` the record and `run/console.rs` the console's own parts, so
    that the machine imports nothing of the mount, the camera or the session and no
    story's diff passes 64 KiB. Instead: one file.
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
27. **A push by hand during a run.** Recommended: when the session's Track view
    carries 038's `push`, the telescope moving by hand or Track following a push, the
    run ends `pushed_away`, saves the stack and asks nothing of the mount, so Track goes
    on following where the observer left it. Subs taken across a push are smeared, and
    the target may no longer be in the frame. Instead: pause the run until the observer
    goes back to the target with Track, then centre again.

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
   while the run holds the camera (FR-021).
3. **Four MCP tools**: `start_imaging_run`, `stop_imaging_run`, `continue_imaging_run`
   and `get_imaging_run`. The first is a moving tool, and `start_motion` learns its name
   beside `set_tracking` (FR-037, FR-038).
4. **The run's `run.json`**, format `imaging-run/1`, in the stack's folder or in
   `runs/<run id>/` under the imaging folder (FR-017).
5. **A public scripted stand-in**, `finder::imaging::run::script` (FR-008, owner
   decision 16).
6. **The Imaging page's run block** and its words, its `DESIGN.md` entry and its
   `.impeccable/design.json` component (FR-035, FR-036).
7. **`crates/finder/README.md`**: three rows of the API table, four tools in
   `## MCP clients`, and a new section `## An imaging run` with a field checklist
   (FR-039 to FR-041).
8. **New test modules are declared in existing test files** (SC-002). Each is one `mod`
   line, in alphabetical order, in its binary's `main.rs`: `imaging_run`,
   `imaging_run_centring`, `imaging_run_docs`, `imaging_run_focus`,
   `imaging_run_limits`, `imaging_run_routes` and `imaging_run_steps` in
   `crates/finder/tests/console/main.rs`; `imaging_run`, `imaging_run_centring` and
   `imaging_run_stack` in `crates/finder/tests/motion/main.rs`; `imaging_run` in
   `crates/finder/tests/mcp/main.rs` and in `crates/finder/tests/cli/main.rs`. No other
   line of those files and no other existing test changes.
9. **Nothing else new.** No dependency, no `Cargo.toml` change, no scope-link message,
   no config key, no console flag, no glossary term and no `unsafe` (SC-003). The
   glossary's **Imaging run** is the plan document's, and **Set point** is 036's.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's scenarios run in
  `npm test`, a later command of the same gate, in `crates/finder/web`.
- **Paused time.** A test of a `Runner` or of the in-process console runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time. A test that
  starts the built `finder` is a plain `#[test]`.
- **The script** is `finder::imaging::run::script::Script` (US1): one stand-in for all
  five parts of a run. Left alone it answers: the target M42 (`id` `NGC 1976`, `name`
  `M42`, RA 83.82°, Dec −5.39°, the script's own constants); a camera and a solver that
  are ready and a lease that is free; a mount that is tracking and steady; imaging
  frames with ids 1, 2, 3 … and the file `frames/2026-01-16/00000N.fits`; a focus check
  that is good with a reading and a record of 3.0″; a solve 0″ from the sensor's centre
  with a tolerance of 90″ that moved the aim point; and a stack that starts in the
  folder `runs/2026-01-16T02-00-10Z-M42`, **the script's stack folder**, and runs with
  nothing stacked or dropped until it is stopped. A test changes one answer at a time:
  `set_mount`, `refuse`, `push_frame`, `push_focus`, `push_solve`, `fail_repoint`,
  `fail_open_aid`, `fail_stack_start`, `fail_stack_stop`, `set_stack` and `end_stack`.
  `set_stack(stacked, dropped, waiting_for_tracker)` sets the stack's counts and whether
  it waits for the tracker, `dropped` a list of reason and count, and never ends the
  stack; `end_stack(failed)` makes it end by itself. `hold(step)` makes every call
  of a step wait, once it is listed, until `release(step)`, which also ends the hold.
  `calls()` is the list of calls the run made, in order, as plan.md § The script words
  them; reading the mount's or the stack's state is not listed.
- **A scripted run** is a `Runner::new(script.steps(), clock)` whose clock is a
  `FixedClock` at Unix 1768528800 (`2026-01-16T02:00:00Z`,
  `crates/finder/src/session.rs:41`), started with
  `runner.start(RunRequest::default(), By::Page)` unless the scenario gives another
  request. **Settling** is sleeping 1 s of virtual time, four of the run's polls, before
  reading `runner.view()`.
- **The straight path** is what `calls()` holds when a scripted run has reached
  `stacking` with nothing changed: `camera.ready`, `solver.ready`, `mount.target`,
  `camera.hold`, `mount.point page`, `camera.take`, `camera.release`, `focus.check`,
  `camera.hold`, `camera.take`, `solver.solve 2`, `camera.release`, `stack.start M42`.
  The first imaging frame has the id 1 and the one that is solved the id 2. The lease
  is given up around the focus check and before the stack, which take it themselves.
- **The scripted console** is 031's console rig (031's spec.md § User Scenarios: a
  `Simulator` at 40° N 75° W on a `VirtualClock` from Unix 1768528800, a `SimNode` with
  the token `test-token`, a paired session, `with_link`, `with_live()` and
  `with_imaging(R)`) with `with_imaging_run_steps(script.steps())` added last. **R** is
  the child path `imaging` of the test's scratch folder.
- **The run rig** is 032's solving rig (032's spec.md § User Scenarios: the same
  simulator with a `SimMount` connected first and `push_to(50.0, 120.0)` after, the
  simulated solver adapter, no aim point) with three changes: the state is built with
  `with_live_solver(Arc::new(crate::support::goto::Stars))` in place of `with_live()`,
  never after it, because each of the two starts a live loop and a GoTo of its own
  (`crates/finder/src/api.rs:247`), so that a finder solve never leaves the paused
  runtime (`crates/finder/tests/support/goto.rs:42`); the session's target is M42; and
  the test waits until the pointing model holds a sync before its steps. Its parts are
  named by each scenario: the console's own from `finder::imaging::run::console`, or
  the script's. The test keeps its own handle of the `SimMount`.
- **The simulator's truth.** The true aim point is `sim.optics().eyepiece_px`, (700,
  420) in the simulated finder camera's frame, which is 29.78″ a pixel (032's spec.md
  § User Scenarios). The true centre of the imaging camera's frame at a time t is
  `sim.imaging_truth_at(t)` (030 US2). **The true offset** is the separation between
  that centre at the console's time and the target's catalogue place, in arcseconds.
  The test computes it with `math_core::angular_distance` over
  `UnitVector::from_radec`, which both work in radians: each RA and Dec goes in as
  `to_radians()`, and the answer comes out as `to_degrees() * 3600.0`. The simulated
  imaging frame is 1236 × 822 pixels at about 2.1845″ a pixel, so 1/20 of its shorter
  side is 89.8″ and the tolerance of owner decision 2 is 90.0″.
- **At rest.** The simulated mount is at rest when `Mount::state` of the test's
  `SimMount` handle reports `moving` false for the azimuth axis and for the altitude
  axis, as `crates/finder/tests/motion/stop/watchdog.rs:121` reads it. **At rest for
  10 s** means that, sampled every 100 ms of virtual time for 10 s, every sample has
  `moving` false for both axes and each axis's `position` is the one of the first
  sample.
- **Scratch folders.** A finder test module takes its folder from
  `crate::support::Scratch::new(name)` (`crates/finder/tests/support/mod.rs:12`), which
  makes the folder under the directory cargo names and removes it when dropped.
- **Requests** go through the router in-process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with `Content-Type:
  application/json` and no `Origin`. A scenario that names an `Origin` builds its
  request itself with a `Host` of localhost on port 8080. A foreign `Origin` is
  `http://elsewhere.example`.
- **Waiting until** a condition means reading `GET /api/imaging` every 100 ms until it
  holds, for at most 900 s of virtual time, and failing otherwise.
- **Comparing a view.** A test compares `run` key by key, never whole, because later
  specs may add keys. `elapsed_s` is compared within 0.5 s, twice the run's poll.
- **Oracles.** Each scenario names its oracle: the simulator's truth; the session; the
  answer of another route of 030 to 036 at the same moment; or, for a scenario that
  checks only what the run says, calls and writes, the words, states, reasons and call
  lists the scenario itself states, which the test compares exactly. A test parses
  `run.json` and `stack.json` itself with `serde_json`, never through the console's
  code. A count or a word has no tolerance; every other tolerance is stated where the
  number is.

### User Story 1 - The run's states, over injected steps (Priority: P1)

As the console, I carry one imaging run from its start to its ending through five parts I
am handed: the mount, the camera, the focus check, the solver and the stack. I say at
every moment which state the run is in and why, and I end it cleanly when the observer
stops it or when the mount's motion ends.

**Why this priority**: Every other story hangs a real part, a route, a page or a tool on
this machine. It is tested with stand-in parts, so it needs none of 030 to 036's parts
to be right; for the run's id it calls only 031's UTC text and 035's rule for a
folder's name.

**Independent Test**: `crates/finder/tests/console/imaging_run.rs`, the module
`imaging_run` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run::`.

**Acceptance Scenarios**:

1. **Given** a script holding `take`, `check`, `solve` and `stack_start`, with its mount
   set to slewing, **When** a test starts a scripted run, settles, sets the mount to
   tracking and steady, settles, and then releases the four steps one at a time in that
   order, settling after each, **Then** a committed test asserts that:
   - the start answers a view whose `id` is `2026-01-16T02-00-00Z-M42`, `by` `page`,
     `target` `{"id":"NGC 1976","name":"M42","ra_deg":83.82,"dec_deg":-5.39}`,
     `started_at` 1768528800.0, `settings`
     `{"subs":null,"duration_s":7200.0,"keep_subs":true}`, `subs`
     `{"stacked":0,"dropped":{}}`, the state `slewing` and no `folder`;
   - the states read at the six settles are, in order, `slewing`, `first_frame`,
     `focus_check`, `solving`, `stacking`, `stacking`, with the messages
     `Slewing to M42.`, `Taking a first imaging frame.`, `Checking focus.`,
     `Solving an imaging frame.` and, twice, `Stacking M42: 0 stacked, 0 dropped.`;
   - at the fifth settle the view has no `folder`, and at the sixth `folder` is the
     script's stack folder, `focus` is
     `{"verdict":"good","reading":3.0,"record":3.0,"unit":"arcsec"}` and `centring` is
     `{"tries":0,"offset_arcsec":0.0,"tolerance_arcsec":90.0}`;
   - `calls()` is exactly the straight path;
   - `finder::imaging::run::label("M42")` is `M42`, `label("NGC 1976")` is `NGC-1976`,
     and `run_id(1768528800.0, "NGC 1976")` is `2026-01-16T02-00-00Z-NGC-1976` (a
     worked example of 035's rule, which `label` calls: every character that is not an
     ASCII letter or digit becomes `-`).
2. **Given** a script, **Then** a committed test asserts that:
   - with `refuse(Part::Camera, "the scope node is not connected")` the start is refused
     with status 409 and those words, `calls()` is exactly `camera.ready`, and `view()`
     is `None`; with `Part::Solver` and words of the test's own, `calls()` is
     `camera.ready`, `solver.ready`; with `Part::Target`, those two and `mount.target`;
     with `refuse(Part::Hold, "the imaging camera is held by the focus aid")`, those
     three and `camera.hold`, and neither `mount.point` nor `camera.release`; with
     `Part::Point`, those four, `mount.point page` and `camera.release`; each refused
     with 409 and the test's words, and each leaving `view()` `None`. The lease is
     asked for after every check that needs no camera and before Track, and a lease
     that was not taken is not given back;
   - a request with `subs` 0 or 100001 is refused with 400 and
     `subs must be a whole number from 1 to 100000`, and one with `duration_s` 0.5,
     43200.5 or not finite with 400 and `duration_s must be a number from 1 to 43200`,
     each with an empty `calls()`;
   - a second start while a run is on is refused with 409 and
     `an imaging run is already running`, and after `runner.stop()` a start is accepted
     and `view()` shows the new run;
   - with `check` held, then
     `refuse(Part::Hold, "the imaging camera is held by a set of darks")`, then `check`
     released, the run ends `camera_failed` with
     `Ended: the imaging camera failed: the imaging camera is held by a set of darks.`
     and `calls()` holds no `solver.solve`: the lease could not be taken back after the
     focus check.
3. **Given** a script whose mount is tracking and not steady, **When** a test starts a
   scripted run and settles, **Then** a committed test asserts the state `tracking`, the
   message `Tracking M42. Waiting for the tracker to be steady.` and neither
   `camera.take` nor `focus.check` in `calls()`; and that after the mount is set steady
   and the test settles, the state is `stacking`.
4. **Given** a scripted run held in turn at `slewing` (the mount set to slewing), at
   `first_frame` (`take` held), at `focus_check` (`check` held), at `solving` (`solve`
   held) and at `stacking`, **When** a test awaits `runner.stop()` in each, **Then** a
   committed test asserts for each that the state is `ended`, `ended.reason` is
   `stopped`, `ended.message` is `Stopped by the observer.`, `ended.at` is
   1768528800.0, and `elapsed_s` no longer rises over 10 s; that the run held at
   `first_frame` called `camera.abort` once; that the run held at `focus_check` called
   `focus.close_aid` once; that the run stopped while stacking called `stack.stop keep`
   once and the others never called `stack.stop`; that each run's last call is
   `camera.release`; that no call is added in the 10 s after the ending; and that
   `calls()` never holds `mount.repoint`.
5. **Given** a scripted run that is stacking, **When** a test sets the mount to each of
   `Ended(Ending::Stopped)`, `Ended(Ending::TooLow)`, `Ended(Ending::SunUp)`,
   `Ended(Ending::LinkLost)`, `Ended(Ending::MountFailed("failed after 5 corrections"))`,
   `Ended(Ending::TargetChanged)` and `Ended(Ending::PushedAway)`, in a run of its own
   each, and settles, **Then** a
   committed test asserts `stack.stop keep` then `camera.release` as the last two calls,
   `stack.stop` made once, and `ended.reason` and `ended.message` equal to, in that
   order:
   `stopped`, `Stopped by the observer.`; `too_low`,
   `Ended: M42 is below the minimum altitude.`; `sun_up`, `Ended: the Sun is up.`;
   `link_lost`, `Ended: the scope link was lost.`; `mount_failed`,
   `Ended: the mount could not reach or follow M42: failed after 5 corrections.`;
   `target_changed`,
   `Ended: the target was changed. Start a new imaging run for the new target.`;
   `pushed_away`,
   `Ended: the telescope was pushed away from M42. Track follows where it is left.`
6. **Given** a scripted run that is stacking, built so that the test holds the only
   `Arc` of the `Runner` and no `Steps` of its own, **When** the test drops that `Arc`
   and sleeps 1 s, **Then** a committed test asserts that `Arc::strong_count` of the
   script is 1 and that, 10 s later, `calls()` is what it was before the drop: a
   dropped runner drops its task and lets go of its parts, and asks nothing of them on
   the way. That is what lets a console dropped in the middle of a run still let go of
   Track, the rule 015 keeps at `crates/finder/tests/motion/tracking/routes.rs:456`.

---

### User Story 2 - Failed steps, counts and warnings, over injected steps (Priority: P1)

As the console, I do not give a run up for one imaging frame the camera drops or one
solve that finds nothing: I ask again, a counted number of times, and only then end the
run and say which part failed. While the stack runs I pass on its counts, I say when the
tracker has paused it, and I keep a warning when focus is slightly soft.

**Why this priority**: A night's first run meets a dropped imaging frame or a field that
does not solve long before it meets a clean path, and the record, the page and the tools
all show what this story computes.

**Independent Test**: `crates/finder/tests/console/imaging_run_steps.rs`, the module
`imaging_run_steps` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_steps::`.

**Acceptance Scenarios**:

1. **Given** a script, **Then** a committed test asserts that:
   - with `push_frame` given `Err(Failure::Camera("boom"))` twice, the run reaches
     `stacking` and `calls()` holds `camera.take` four times: three for the first
     imaging frame and one for the solve;
   - with it given three times, the run ends `camera_failed` with
     `Ended: the imaging camera failed: boom.` after exactly three `camera.take`, and
     `calls()` holds no `focus.check`;
   - with `Err(Failure::Save("could not save the imaging frame: disk"))` once, the run
     ends `save_failed` at the first take, with
     `Ended: the console could not save to its imaging folder: could not save the imaging frame: disk. Check the disk.`
2. **Given** a script, **Then** a committed test asserts that:
   - with `push_solve` given `Err(Failure::NotSolved("the imaging solver found no solution"))`
     twice, the run reaches `stacking`, and `calls()` after `focus.check` is
     `camera.hold`, `camera.take`, `solver.solve 2`, `camera.take`, `solver.solve 3`,
     `camera.take`, `solver.solve 4`, `camera.release`, `stack.start M42`: the focus
     check is not repeated;
   - with it given three times, the run ends `solver_failing` with
     `Ended: the imaging solver keeps failing: the imaging solver found no solution.`;
   - with `Err(Failure::SolverUnavailable("the imaging solver has no star database it can read"))`
     once, the run ends `solver_unavailable` at the first solve, with
     `Ended: the imaging solver cannot run: the imaging solver has no star database it can read.`
3. **Given** a scripted run that is stacking, **Then** a committed test asserts that:
   - after `set_stack(5, &[("not_solved", 2)], false)` and a settle, `subs` is
     `{"stacked":5,"dropped":{"not_solved":2}}` and the message is
     `Stacking M42: 5 stacked, 2 dropped.`;
   - after `set_stack(5, &[("not_solved", 2)], true)` and a settle the state is
     `paused_by_tracker` and the message `Paused by the tracker: 5 stacked so far.`,
     and after `set_stack(5, &[("not_solved", 2)], false)` and a settle the state is
     `stacking` again;
   - a run whose `push_focus` gave a `Verdict::SlightlySoft` check with reading 3.6 and
     record 3.1 in arcseconds reaches `stacking` with `warnings` equal to
     `["Focus is slightly soft: stars are 3.6″ across against a record of 3.1″."]` and
     `focus.verdict` `slightly_soft`; and one given reading 6.6 and record 5.6 in
     pixels has the warning
     `Focus is slightly soft: stars are 6.6 px across against a record of 5.6 px.` and
     `focus.unit` `px`.
4. **Given** a script, **Then** a committed test asserts that:
   - with
     `fail_stack_start(Failure::Save("cannot make the stack folder: no space left on device"))`
     the run ends `save_failed` with
     `Ended: the console could not save to its imaging folder: cannot make the stack folder: no space left on device. Check the disk.`,
     never calls `stack.stop`, and has no `folder`;
   - with `fail_stack_start(Failure::Other("the stack is busy"))` it ends `stack_failed`
     with `Ended: the stack stopped: the stack is busy.`, and `calls()` ends
     `stack.start M42`, `camera.release`;
   - with
     `fail_stack_stop(Failure::Other("could not remove subs/000004.fits"))`
     a run stopped while stacking still ends `stopped`, with `warnings`
     `["The stack did not end cleanly: could not remove subs/000004.fits."]`.

---

### User Story 3 - Start, stop and read a run over HTTP, and its record (Priority: P1)

As the observer, I start an imaging run with one request, read where it has got to in
the session, stop it with another, and find afterwards a file that says what the run
did.

**Why this priority**: The page and the MCP tools are both thin callers of these three
routes.

**Independent Test**: `crates/finder/tests/console/imaging_run_routes.rs`, the module
`imaging_run_routes` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_routes::`.

**Acceptance Scenarios**:

1. **Given** the scripted console, **When** a test sends `POST /api/imaging/run` with
   `{}` and waits until `run.state` is `stacking` and `run.folder` is present, **Then**
   a committed test asserts that:
   - the answer is 200 and an imaging view whose `run.by` is `page` and whose `run.id`
     begins `2026-01-16T02-` and ends `-M42`;
   - `GET /api/session` carries the same `run` under `imaging`, key for key but for
     `elapsed_s`, and `session.json`, where the console keeps one, holds no `run` key;
   - `run.folder` is the script's stack folder, and R joined with it holds `run.json`
     with `format` `imaging-run/1`, `finder` equal to the `version` of
     `GET /api/health`, `id`, `by`, `target`, `started_at` and `settings` equal to the
     view's, `started_utc` equal to the id's first twenty characters with the two `-`
     of its time made `:`, `state` `stacking`, and `events` whose `state`s are
     `slewing`, `first_frame`, `focus_check`, `solving`, `stacking`, in that order,
     with `at` never falling. The script's mount is steady from the start, so the run
     never shows `tracking`.
2. **Given** the scripted console, **Then** a committed test asserts that:
   - `POST /api/imaging/run` with `{"subs":0}` and `{"duration_s":0}` each answers 400
     with the words of US1-S2, and with `{"subs":1.5}`, `{"duration_s":"long"}` and
     `{"exposure_ms":1}` each answers 400; with a `Content-Type` of `text/plain` it
     answers 415; with a foreign `Origin` 403; each with a JSON `error`, after which
     `GET /api/imaging` has no `run` key and R holds no `runs` folder;
   - with `refuse(Part::Solver, "this console has no imaging solver")` it answers 409
     with exactly that `error`, and R holds no `runs` folder;
   - a second `{}` while a run is on answers 409 and
     `an imaging run is already running`;
   - a console built the same way without `with_imaging_run_steps` answers each of the
     three routes with 409 and `this console runs no imaging runs`.
3. **Given** the scripted console with a run that is stacking, after
   `set_stack(4, &[], false)` and a settle, **When** a test sends
   `DELETE /api/imaging/run`, **Then** a committed test asserts that the answer is 200
   with `run.state` `ended` and `run.ended.reason` `stopped`; that `run.json` then has
   `state` `ended`, an `ended` equal to the view's, `subs.stacked` 4 and `ended` as the
   last of its `events`; that a second `DELETE`
   answers 200 with the same `run`; and that on a console where no run was ever started
   `DELETE` answers 200 with no `run` key.
4. **Given** the scripted console, **Then** a committed test asserts that
   `POST /api/imaging/run/continue` with `{"anyway":true}` answers 409 and
   `no imaging run is running` before any run, and 409 and
   `the imaging run is not paused` while one is stacking; that `{}`, `{"anyway":"yes"}`
   and `{"anyway":true,"x":1}` answer 400; and that a foreign `Origin` answers 403.
5. **Given** the scripted console with a run that is stacking, **When** a test sets the
   script's mount to `Ended(Ending::LinkLost)`, makes the simulator unresponsive
   (`Simulator::set_unresponsive`, `crates/simulator/src/controls.rs:115`) and waits
   until the imaging view's `state` is `no_link`, **Then** a committed test asserts that
   the view still carries `run`, with `state` `ended` and `ended.reason` `link_lost`.
6. **Given** the scripted console with the script holding `take`, **When** a test reads
   the console's `time` from `GET /api/session`, sleeps the virtual time left to the
   next whole second and 50 ms more, starts a run, waits until its `run.state` is
   `first_frame`, stops it with `DELETE /api/imaging/run`, starts another, waits the
   same way and stops that, and then reads `time` again and finds it in the same whole
   second (the test fails otherwise), **Then** a committed test asserts that the two
   runs have the same `id`; that neither run stacked; that neither view had a `folder`
   before its stop; that after its stop the first run's
   `folder` is `runs/` followed by its `id`; that the second's is the same with `-2`
   added; and that each folder holds a `run.json` whose `ended.reason` is `stopped` and
   whose `events` are `slewing`, `first_frame`, `ended`. **Given** instead a console
   whose R already holds a regular file named `runs`, **When** a test starts a run and
   waits until it has ended, **Then** the committed test asserts `ended.reason`
   `save_failed`, an `ended.message` beginning
   `Ended: the console could not save to its imaging folder:`, and that `calls()` ends
   `stack.start M42`, `stack.stop keep`, `camera.release`.
7. **Given** the run of US3-S3 after its stop, **Then** a committed test asserts that
   `run.json` contains none of `test-token`, `scope.local` and `lat_deg`, and has no
   `message`, `elapsed_s` or `site` key.

---

### User Story 4 - Pointing through Track, on the console's own mount, camera and solver parts (Priority: P1)

As the observer, I start a run and the console points the telescope by asking Track, as
I would from the page, holds the imaging camera while it does, and solves what the
camera sees. When I press Stop, or a guard ends the motion, the run ends with it and
never restarts the mount.

**Why this priority**: This is where the run first moves real metal: the three parts
through which it meets Track, 031's imaging camera and 032's imaging solver.

**Independent Test**: `crates/finder/tests/motion/imaging_run.rs`, the module
`imaging_run` of finder's `motion` test binary, on the run rig. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test motion imaging_run::`.

In this story **the centred rig** is the run rig with the console's own mount, camera
and solver parts, the script's focus and stack parts, and the session's aim point set
first to the true aim point with `Session::set_aim`. **A centred run** is a run started
on it with `POST /api/imaging/run` and `{}`, on which the test has waited until
`run.state` is `stacking`. No scenario of this story re-centres: the machine gains that
in US5.

**Acceptance Scenarios**:

1. **Given** the centred rig, **When** a test sends `POST /api/imaging/run` with `{}`
   and waits until `run.state` is `stacking`, sampling the session every 100 ms,
   **Then** a committed test asserts, against the simulator's truth and the session,
   that:
   - the start answered 200 with `run.state` `slewing`, and the session's `motion` then
     read `by` `page` and `tool` `imaging_run`;
   - `run.centring.tries` is 0, `run.centring.tolerance_arcsec` is 90.0 within 0.5, and
     `run.centring.offset_arcsec` is at most `run.centring.tolerance_arcsec`: Track
     brought the target inside the tolerance with no re-centre, which is also what
     keeps this scenario true once US5 adds the centring loop;
   - the true offset, read when the test first sees `stacking`, is at most 110″: the
     tolerance and 20″ more, which allows the tracker's 10″ deadband twice over
     (`crates/finder/src/tracking.rs:31`); and `run.centring.offset_arcsec` is within
     30″ of that true offset, the deadband three times over;
   - `track.state`, once it first read `tracking`, never read anything else.
2. **Given** the centred rig, **Then** a committed test asserts that
   `POST /api/imaging/run` with `{}` answers 409: with `no target is selected` after
   `PUT /api/session` with `{"target":null}`; with the target set to M8, which
   `finder::pushto::class` gives as below the horizon there and then, as
   `crates/finder/tests/motion/tracking/routes.rs:182` checks it, with an `error`
   containing `M8 is below the horizon`, Track's own words; and, after
   `PUT /api/imaging/settings` has set the `roi` `{"x":0,"y":0,"width":412,"height":274}`,
   the top-left of 033's nine tiles of the simulated sensor, with
   `the imaging camera has a region of interest set: clear it before an imaging run`.
   After each the session has no `track` that is wanted, the imaging view has no `run`,
   R holds no `runs` folder, and, once the region is cleared, `POST /api/imaging/frame`
   answers 200: the lease was given back (oracle: 031's own route).
3. **Given** a centred run, **When** a test sends `POST /api/stop`, **Then** a committed
   test asserts that the run ended `stopped`, that the script's `calls()` hold
   `stack.stop` once, and that from 2 s after the stop the simulated mount is at rest
   for 10 s (oracle: the simulator's mount).
4. **Given** a centred run, on a rig of its own each, **Then** a committed test asserts
   that:
   - after `PUT /api/session` with `{"min_alt_deg":80.0}`, as
     `crates/finder/tests/motion/tracking/min_alt.rs:67` raises it, the run ends
     `too_low` and the simulated mount is at rest 2 s later;
   - after `Simulator::set_unresponsive(true)`, the run ends `link_lost`;
   - after `PUT /api/session` with `{"target":"M45"}`, the run ends `target_changed`
     and `track.state` is still `tracking` with `track.target.messier` `M42`;
   - after `rig.mount.push_by(0.0, -2.0)` (`crates/simulator/src/mount.rs:269`), down and
     so behind the rising target, while
     the run stacks, the run ends `pushed_away` within 60 s, and `track.state` is still
     `tracking` with `track.push.state` `moving` or `following` (oracle: the session,
     whose `push` is 038's);
   - on a centred rig whose clock starts 120 s before the Sun's centre reaches −0.83°
     at the site, found by the test by bisection of `crate::support::goto::sun_truth`
     (`crates/finder/tests/support/goto.rs:91`), with the target M81, a run started at
     once ends `sun_up`, in whatever state it had reached, and the simulated mount is at
     rest 2 s later;
   - on a centred rig, right after a start is answered and while `run.state` is
     `slewing`, `POST /api/imaging/frame` answers 409 with the `error`
     `the imaging camera is held by the imaging run`; and after `POST /api/stop` has
     ended that run it answers 200 (oracle: 031's own route under 031 US9's lease,
     whose refusal names the holder).
5. **Given** the centred rig and its mount part alone, made by the test with
   `finder::imaging::run::console::mount`, no run being started, **Then** a committed
   test asserts that:
   - `point(By::Page)` answers `Ok(())`, and `state()`, read every 100 ms, is
     `MountState::Tracking { steady: true }` within 900 s;
   - `repoint()` then answers `Ok(())`; `state()` is `Tracking { steady: true }` again
     within 120 s; and the session's `motion` then reads `by` `page` and `tool`
     `imaging_run` (oracle: the session);
   - after `POST /api/stop`, `state()` is `Ended(Ending::Stopped)`; `repoint()` answers
     `Err(Failure::Stopped)`; the session's `track.state` is `off` right after it; and
     the simulated mount is at rest for 10 s: a re-centre asked for after a Stop turns
     nothing on (oracle: the session and the simulator's mount).

---

### User Story 5 - Centring on the sensor (Priority: P1)

As the observer, I start a run on a telescope whose imaging camera the console has never
related to the finder camera, and the run brings the target to the middle of the imaging
camera's sensor by asking Track again with the aim point it has just measured. If I
press Stop while it does so, Stop still wins.

**Why this priority**: This is where "nothing to tap" is proven.

**Independent Test**: `crates/finder/tests/console/imaging_run_centring.rs`, the module
`imaging_run_centring` of `console`, for the scripted scenarios US5-S1 to US5-S3, and
`crates/finder/tests/motion/imaging_run_centring.rs`, the module `imaging_run_centring`
of `motion`, for US5-S4 and US5-S5 on the run rig. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console imaging_run_centring::`
and `cargo nextest run -p finder --locked --test motion imaging_run_centring::`.

**Acceptance Scenarios**:

1. **Given** a script whose `push_solve` gives
   `Centre { offset_arcsec: 400.0, tolerance_arcsec: 90.0, aim_moved: true }` and then
   the same with `offset_arcsec` 50.0, and which holds `repoint`, **When** a test
   starts a scripted run and settles, **Then** a committed test asserts the state
   `centring`, the message
   `Centring M42: 400″ from the sensor's centre, try 1 of 3.` and `centring`
   `{"tries":1,"offset_arcsec":400.0,"tolerance_arcsec":90.0}`; and, after `repoint` is
   released and the test settles, the state `stacking`, `centring`
   `{"tries":1,"offset_arcsec":50.0,"tolerance_arcsec":90.0}`, and `calls()` after
   `solver.solve 2` equal to `mount.repoint`, `camera.take`, `solver.solve 3`,
   `camera.release`, `stack.start M42`.
2. **Given** a script whose `push_solve` gives an offset of 400″ with `aim_moved` true
   four times, **When** a test starts a scripted run and settles, **Then** a committed
   test asserts that the run ended `not_centred` with
   `Ended: M42 is still 400″ from the sensor's centre after 3 tries.`, that `calls()`
   holds `mount.repoint` three times and `solver.solve` four times, and that it holds
   no `stack.start`.
3. **Given** a script, **Then** a committed test asserts that:
   - an offset of 400″ with `aim_moved` false, then 50″, reaches `stacking` with
     `centring.tries` 1 and no `mount.repoint` in `calls()`;
   - an offset of 400″ followed by `fail_repoint(Failure::Mount("the mount is already moving"))`
     ends `mount_failed` with
     `Ended: the mount could not reach or follow M42: the mount is already moving.`;
   - an offset of 400″ followed by `fail_repoint(Failure::Stopped)` ends `stopped`;
   - after an offset of 400″, with the mount set to tracking and not steady while
     `repoint` is held, no `camera.take` follows `mount.repoint` in `calls()` 10 s
     after `repoint` is released, and one follows once the mount is set steady and the
     test settles;
   - after an offset of 400″, with `repoint` held, a `runner.stop()` awaited beside a
     sleep of 10 s has not answered when the sleep ends; once `repoint` is released it
     answers a view that ended `stopped`, and `calls()` holds no `camera.take` after
     `mount.repoint`: a re-centre is never abandoned half-way, and a stop that arrives
     during one is acted on as soon as it has answered (FR-005).
4. **Given** the run rig with the console's own mount, camera and solver parts and the
   script's focus and stack parts, and no aim point, **When** a test sends
   `POST /api/imaging/run` with `{}` and waits until `run.state` is `stacking`, **Then**
   a committed test asserts, against the simulator's truth, that:
   - `run.centring.tries` is at least 1 and `run.centring.tolerance_arcsec` is 90.0
     within 0.5;
   - the true offset, read when the test first sees `stacking`, is at most the
     tolerance plus 20″, which allows the tracker's 10″ deadband twice over
     (`crates/finder/src/tracking.rs:31`);
   - the session's `aim.source` is `measured` and its `aim` is within 2.0 pixels of the
     true aim point in each axis: the 1.5 pixels 032 allows the same oracle with the
     mount at rest (032 US4-S1), and 0.5 pixel more for the tracker's 10″ deadband,
     which is 0.34 pixel at 29.78″ a pixel.
5. **Given** the run rig with the console's own mount and camera parts and the script's
   solver, focus and stack parts, the script's solver answering an offset of 400″ with
   `aim_moved` true every time, **When** a test sends `POST /api/imaging/run` with
   `{}`, reads `GET /api/imaging` every 100 ms and sends `POST /api/stop` as soon as it
   reads the state `centring`, **Then** a committed test asserts that the run ended
   `stopped` and not `mount_failed`, wherever in the re-centre the Stop landed; that
   the script's `calls()` hold no `stack.start`; and that from 2 s after the stop the
   simulated mount is at rest for 10 s: no re-centre restarts the mount (oracle: the
   simulator's mount).

---

### User Story 6 - The focus gate and its pause (Priority: P1)

As the observer at a hand focuser, I am stopped before the run wastes a solve on a
picture that is clearly out of focus. The focus aid opens, I turn the knob, and I ask the
run to check again, or to go on anyway.

**Why this priority**: With a manual focuser the first imaging frame of a night is often
far out, and an unfocused imaging frame does not solve.

**Independent Test**: `crates/finder/tests/console/imaging_run_focus.rs`, the module
`imaging_run_focus` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_focus::`.

**Acceptance Scenarios**:

1. **Given** a script whose `push_focus` gives `Verdict::ClearlyOut` with reading 9.8 and
   record 3.1 in arcseconds, **When** a test starts a scripted run and settles, **Then**
   a committed test asserts the state `focus_pause`, `focus`
   `{"verdict":"clearly_out","reading":9.8,"record":3.1,"unit":"arcsec"}`, the message
   `Focus is clearly out: stars are 9.8″ across against a record of 3.1″. Turn the focuser, then check focus again or go on anyway.`,
   `calls()` ending `camera.take`, `camera.release`, `focus.check`, `focus.open_aid`,
   and no `solver.solve` in it. **Given** instead each of these checks, **Then** the committed
   test asserts a pause with these messages:
   - `Verdict::ClearlyOut` with reading 21.0 in pixels and no record:
     `Focus is clearly out: stars are 21.0 px across. Turn the focuser, then check focus again or go on anyway.`;
   - `Verdict::Unmeasurable` with no reading:
     `Focus could not be measured. Turn the focuser, then check focus again or go on anyway.`;
   - `Verdict::NoRecord` with reading 3.2 in arcseconds:
     `There is no focus record for this camera yet. Focus with the focus aid, then check focus again or go on anyway.`;
   - `Err(Failure::Other("the focus check ended: no_region"))`: the message of
     `Verdict::Unmeasurable`, `focus.verdict` `unmeasurable`, and `warnings`
     `["The focus check did not finish: the focus check ended: no_region."]`.
2. **Given** the paused run of US6-S1, **When** a test calls `runner.resume(false)` and
   settles, **Then** a committed test asserts that `calls()` goes on with
   `focus.close_aid`, `focus.check`, `camera.hold`, `camera.take`, `solver.solve 2`, and
   that the state is `stacking`. **Given** a script whose `push_focus` gives `Verdict::ClearlyOut`
   twice, **Then** the committed test asserts that after `resume(false)` the state is
   `focus_pause` again and `focus.open_aid` was called twice.
3. **Given** the paused run of US6-S1, **When** a test calls `runner.resume(true)` and
   settles, **Then** a committed test asserts that `calls()` goes on with
   `focus.close_aid`, `camera.hold`, `camera.take`, `solver.solve 2`, with no second
   `focus.check`;
   that the state is `stacking`; and that `warnings` is
   `["Focus was not checked again: the observer went on anyway."]`.
4. **Given** the paused run of US6-S1, **Then** a committed test asserts that 899 s
   after the start the state is still `focus_pause`; that 2 s later the run has ended
   `focus_timeout` with `Ended: focus was left paused for 15 minutes.`; that
   `focus.close_aid` was called once; and that `calls()` holds no `stack.start`.
5. **Given** the paused run of US6-S1, in a run of its own each, **Then** a committed
   test asserts that `runner.stop()` ends it `stopped`, and the mount set to
   `Ended(Ending::TooLow)` ends it `too_low`, each with `focus.close_aid` called once
   before `camera.release`; that with
   `fail_open_aid(Failure::Other("the focus aid is busy"))` the run still pauses, with
   `warnings` `["The focus aid could not be opened: the focus aid is busy."]`, and a
   `runner.stop()` on that run still calls `focus.close_aid` once, before
   `camera.release`: the aid is closed on every way out of a pause, whether or not it
   could be opened; and that `resume(true)` on a run that is stacking is refused with
   409 and `the imaging run is not paused`.
6. **Given** the scripted console with a run paused as in US6-S1, **When** a test sends
   `POST /api/imaging/run/continue` with `{"anyway":false}`, **Then** a committed test
   asserts 200 and, once settled, `run.state` `stacking`; and that on another such run
   `{"anyway":true}` answers 200 and leaves the warning of US6-S3 in `run.warnings`.
7. **Given** 033's focus rig (033's spec.md § User Scenarios) with the simulated solver
   adapter as its solver, on which the test has first run the focus aid at the
   simulator's focus error 0.0, with `POST /api/imaging/focus` and `{}`, until the
   imaging view's `focus.record` holds tonight's record, and stopped it with
   `DELETE /api/imaging/focus`; and a `Runner` whose camera and focus parts are the
   console's own and whose mount, solver and stack parts are the script's, **Then** a
   committed test asserts, against the simulator's focus error
   (`Simulator::set_focus_error`, 033 US1), that:
   - at the focus error 0.0 a run reaches `stacking` with `focus.verdict` `good` and
     `focus.reading` at most 1.10 times `focus.record`, 033's bound for good;
   - with the focus error raised from 0.0 in steps of 1.0, at most ten of them, until a
     check the test starts itself, with `POST /api/imaging/focus` and
     `{"check":true}`, ends with the verdict `clearly_out` (the test fails if none has
     by the focus error 10.0), a run pauses with `focus.verdict` `clearly_out`, and the
     imaging view's `focus.aid` has `state` `running` and `check` false: the pause
     opened the focus aid;
   - with the focus error then set back to 0.0, `resume(false)` reaches `stacking`, and
     `focus.aid.state` is `ended`;
   - a start while a focus aid the test started is running is refused with 409 and the
     `error` `the imaging camera is held by the focus aid`, the script's `calls()` hold
     no `mount.point`, and the imaging view's `focus.aid.state` is still `running`
     (oracle: 031 US9's lease, whose refusal names its holder, and 033's name for the
     aid; the camera part asks for the lease before it looks for an imaging frame in
     flight, FR-021);
   - on a focus rig whose R is fresh, so that the camera has no focus record, a run
     pauses with `focus.verdict` `no_record`.

---

### User Story 7 - Stacking to a limit, and the endings a stack can cause (Priority: P1)

As the observer, I ask for fifty subs or twenty minutes and walk away. The run stops the
stack at the limit and saves it. If the camera fails while it stacks, or the sky clouds
over and sub after sub is dropped, the run ends and says why, and what it had stacked is
saved.

**Why this priority**: Without a limit a forgotten run writes subs until a guard or the
disk ends it, and without these endings a stack that has stopped by itself leaves a run
that says it is still stacking.

**Independent Test**: `crates/finder/tests/console/imaging_run_limits.rs`, the module
`imaging_run_limits` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_limits::`.

**Acceptance Scenarios**:

1. **Given** a scripted run started with `subs` 3 that is stacking, **When** a test calls
   `set_stack(2, &[], false)` and settles, and then `set_stack(3, &[], false)` and
   settles, **Then** a committed test asserts `stacking` after the first and, after the
   second, an ending `count_limit` with
   `Finished: 3 subs stacked, the number asked for.` and `stack.stop keep` called once.
   **Given** the same started with `keep_subs` false, **Then** the committed test
   asserts `stack.stop discard` called once and never `stack.stop keep`.
2. **Given** a script holding `solve`, and a scripted run started with `duration_s` 60,
   **When** a test waits 100 s, releases `solve`, and settles, **Then** a committed test
   asserts that 58 s later the state is still `stacking`, and that 3 s after that the
   run has ended `duration_limit` with `Finished: the 60 s limit was reached.` and
   `stack.stop keep` called once: the time before stacking does not count. **Given**
   the same, with `set_stack(0, &[], true)` called right after `solve` is released,
   **Then** the committed test asserts that 58 s later the state is `paused_by_tracker`
   and that 3 s after that the run has ended `duration_limit`: the time the tracker
   pauses the stack does count.
3. **Given** a scripted run that is stacking, **When** a test calls, settling after each,
   `set_stack(4, &[], false)`, `set_stack(4, &[("not_solved", 4)], false)`,
   `set_stack(4, &[("not_solved", 7)], false)` and
   `set_stack(4, &[("not_solved", 7), ("too_few_stars", 3)], false)`, **Then** a
   committed test asserts `stacking` after each of the first three and, after the
   fourth, an ending `subs_failing` with
   `Ended: the last 10 subs were all dropped (not_solved 7, too_few_stars 3).`
   **Given** instead, settling after each, `set_stack(4, &[], false)`,
   `set_stack(4, &[("not_solved", 9)], false)`,
   `set_stack(5, &[("not_solved", 9)], false)` and
   `set_stack(5, &[("not_solved", 18)], false)`, **Then** the committed test asserts
   the state is still `stacking`: a stacked sub starts the count of drops again.
4. **Given** a scripted run that is stacking, in a run of its own each, **When** a test
   calls `end_stack(Some(failure))` and settles, **Then** a committed test asserts, for
   `Failure::Camera("no imaging frame header arrived within 152 s")`,
   `Failure::Save("no space left on device")`,
   `Failure::SolverUnavailable("the imaging solver has no star database it can read")`,
   `Failure::NotSolved("the imaging solver found no solution")` and
   `Failure::Other("the stack's grid could not be built")`, the reasons `camera_failed`,
   `save_failed`, `solver_unavailable`, `solver_failing` and `stack_failed`, the last
   with `Ended: the stack stopped: the stack's grid could not be built.`; that after
   `end_stack(None)` and a settle a run has ended `stopped`; and that each of the six
   runs called `stack.stop keep` once, so that the stack is saved whoever ended it.

---

### User Story 8 - The whole run on the console's own parts, and in the binary (Priority: P1)

As the observer, I ask for the Orion Nebula and three subs, and the console I started
from the command line does all of it with its own five parts: it tracks, checks focus,
centres, stacks, stops at three, and leaves the stack and the run's record in one
folder, with Track still on.

**Why this priority**: This story joins the last part, the stack, and makes the binary
run the whole procedure.

**Independent Test**: `crates/finder/tests/motion/imaging_run_stack.rs`, the module
`imaging_run_stack` of `motion`, for US8-S1 and US8-S2 on the run rig, and
`crates/finder/tests/cli/imaging_run.rs`, the module `imaging_run` of `cli`, for US8-S3.
The `test` gate's `cargo nextest run --workspace --all-targets --all-features --locked`
runs both; during development,
`cargo nextest run -p finder --locked --test motion imaging_run_stack::` and
`cargo nextest run -p finder --locked --test cli imaging_run::`.

**Acceptance Scenarios**:

1. **Given** the run rig built with `with_imaging_run()`, so that every part is the
   console's own, on which the test has first run 033's focus aid until the camera has
   a focus record and stopped it, **When** a test sends `POST /api/imaging/run` with
   `{"subs":3}` and waits until `run.state` is `ended`, **Then** a committed test
   asserts that:
   - `run.ended.reason` is `count_limit` and `run.subs.stacked` is at least 3;
   - `run.folder` is `runs/` followed by the imaging view's `stack.id`, which ends
     `-M42`, and R joined with it holds `run.json`, whose `ended.reason` is
     `count_limit`, and 035's `stack.json`, `stack.fits` and `stack.png`;
   - that `stack.json` lists at least three subs with the `outcome` `stacked`, and the
     `file` of each, a path inside the run's folder, exists there;
   - 2 s after the ending the session's `track.state` is `tracking` with
     `track.target.messier` `M42`: the run stopped nothing (oracle: the session).
2. **Given** the same rig, **When** a test sends `{"subs":2,"keep_subs":false}` and
   waits until the run has ended, **Then** a committed test asserts `count_limit`, a
   `stack.fits` in the run's folder, and that no `file` that its `stack.json` lists
   exists in the run's folder any more, nor that file's name with `.fits` made
   `.solve.json`.
   **Given** the same rig with a run that is stacking, **When** a test sends
   `DELETE /api/imaging/run`, **Then** the committed test asserts `stopped`, the
   imaging view's `stack.state` `ended`, a `run.json` in the run's folder,
   `track.state` still `tracking`, and that `POST /api/imaging/frame` then answers 200.
3. **Given** the built `finder` binary run with `--simulate`, `--state S` and `--listen`
   on port 0 of `127.0.0.1`, S a fresh scratch folder, **When** a test reads
   `GET /api/imaging` every 200 ms, for at most 60 s, until its `state` is `ready`, and
   then sends `POST /api/imaging/run` with `{}`, no target being set, **Then** a
   committed test asserts 409 with the `error` `no target is selected`: the binary runs
   imaging runs, or the answer would be `this console runs no imaging runs`.

---

### User Story 9 - The run on the Imaging page (Priority: P2)

As the observer with a phone in the dark, I open the Imaging page, press one button, and
read one line that says what the run is doing, how many subs are stacked and dropped,
and who started it. At a focus pause the same block offers the two ways on.

**Why this priority**: At the telescope the page is the only thing to hand.

**Independent Test**: `npm test` in `crates/finder/web`, a command of the `test` gate,
runs `src/components/ImagingRunBlock.test.tsx`.

**Acceptance Scenarios**:

1. **Given** `ImagingRunBlock` rendered with a `ready` imaging view and no `run`,
   **Then** a committed test asserts one region named `Imaging run` holding a button
   `Start imaging run`, fields labelled `Subs to stack` and `Minutes to stack`, both
   empty, a checked checkbox `Keep every sub`, and no `status` or `alert`. **When** the
   test clicks the button, with `fetch` answering 200 and a view, **Then** the committed
   test asserts that `fetch` was called with `/api/imaging/run` and
   `{"method":"POST","headers":{"Content-Type":"application/json"},"body":"{\"keep_subs\":true}"}`,
   then `refresh`; that with `50` and `30` typed and the checkbox cleared the body is
   `{"subs":50,"duration_s":1800,"keep_subs":false}`; and that a 409 answer with
   `{"error":"no target is selected"}` shows an alert with exactly that text and calls
   no `refresh`.
2. **Given** the block with a `run` whose `state` is `stacking`, `message`
   `Stacking M42: 12 stacked, 3 dropped.`, `subs`
   `{"stacked":12,"dropped":{"not_solved":2,"too_few_stars":1}}`, `elapsed_s` 125.4 and
   `by` `page`, **Then** a committed test asserts a status reading exactly
   `Stacking M42: 12 stacked, 3 dropped.`, the text
   `12 stacked · 3 dropped (not solved 2, too few stars 1)`, the text "Elapsed 2:05", a
   button `Stop imaging run` that calls `fetch` with `/api/imaging/run` and
   `{"method":"DELETE"}` and then `refresh`, no `Start imaging run` button, and no text
   `Started by an MCP client`. With `by` `mcp` the committed test asserts the text
   `Started by an MCP client`.
3. **Given** the block with a `run` whose `state` is `focus_pause` and whose `message`
   is that of US6-S1, **Then** a committed test asserts an alert reading exactly that
   message, a button `Check focus again` that posts `{"anyway":false}` to
   `/api/imaging/run/continue`, a button `Go anyway` that posts `{"anyway":true}`, and
   the button `Stop imaging run`.
4. **Given** the block with a `run` whose `state` is `centring`, whose `message` is that
   of US5-S1 and whose `warnings` hold the arcsecond warning of US2-S3, **Then** a
   committed test asserts a status reading exactly the message and a paragraph reading exactly
   the warning.
5. **Given** the block with an ended `run` whose `ended` is
   `{"reason":"stopped","at":1768529000,"message":"Stopped by the observer."}`, with 12
   stacked and the folder `runs/2026-01-16T02-00-10Z-M42`, **Then** a committed test
   asserts a status reading exactly
   `Stopped by the observer. 12 subs stacked · saved in runs/2026-01-16T02-00-10Z-M42.`,
   a `Start imaging run` button and no alert; with 0 stacked, the status
   `Stopped by the observer. No sub was stacked.`; and with the reason `sun_up` and the
   message `Ended: the Sun is up.`, an alert reading
   `Ended: the Sun is up. 12 subs stacked · saved in runs/2026-01-16T02-00-10Z-M42.`
   and no status.
6. **Given** 031's `ImagingPage` fed by `useImaging`, as 031's own page test feeds it,
   with `fetch` answering every read with a `ready` view holding the `run` of US9-S2,
   **Then** a committed test asserts that the region named `Imaging run` comes before
   the region named `Camera` in document order and shows the status of US9-S2; and that
   with a view whose `state` is `no_link` and which holds the ended `run` of US9-S5 the
   region still shows its status, while with `no_link` and no `run` the page has no
   region named `Imaging run`.
7. **Given** `DESIGN.md` and `.impeccable/design.json`, read by the test with
   `readWebFile` (`crates/finder/web/src/test/files.ts:11`) and the paths
   `../../../DESIGN.md` and `../../../.impeccable/design.json`, **Then** a committed
   test asserts that `DESIGN.md` holds a line that is exactly
   `### Imaging run — built`, and that the `components` array of the JSON holds exactly
   one entry whose `name` is `Imaging run`, with the `kind` `custom` and the `refersTo`
   `button` (oracle: the words of plan.md § `DESIGN.md` and `.impeccable/design.json`).

---

### User Story 10 - Four MCP tools (Priority: P2)

As an MCP client, I set the target, start an imaging run, read it until it stacks, and
stop it. I pass the same guards as the page, and I cannot ask for any one step out of
turn.

**Why this priority**: "Ask for the Orion Nebula and get a picture" is asked by voice or
chat as often as by a tap.

**Independent Test**: `crates/finder/tests/mcp/imaging_run.rs`, the module `imaging_run`
of finder's `mcp` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test mcp imaging_run::`.

**Acceptance Scenarios**:

1. **Given** the scripted console served through the MCP door, as
   `crates/finder/tests/support/mcp.rs:28` serves a state, **When** a committed test
   reads `tools/list`, **Then** it asserts that the list holds `start_imaging_run`,
   `stop_imaging_run`, `continue_imaging_run` and `get_imaging_run`, each with the
   description plan.md § The MCP tools gives and an `outputSchema` of
   `{"type":"object"}`; that only `get_imaging_run` carries `readOnlyHint: true`; and
   that the input schema of `start_imaging_run` names `subs`, `duration_s` and
   `keep_subs`, and that of `continue_imaging_run` names `anyway`.
2. **Given** the same console with a call log the test reads, **When** a committed test
   calls `get_imaging_run`, then `start_imaging_run` with `{"subs":5}`, then
   `get_imaging_run`, **Then** it asserts that the first answers `{"run":null}`; that
   the start answers a `run` with `by` `mcp` and `settings.subs` 5; that the last
   answers the same `run`, key for key but for `elapsed_s`, `state`, `message`, `focus`,
   `centring` and `folder`; that the script's `calls()` hold `mount.point mcp`; and
   that the log holds exactly one line, beginning
   `mcp start_imaging_run {"subs":5} -> `.
3. **Given** the same console, **Then** a committed test asserts tool errors with these
   words: `start_imaging_run` with `{"subs":0}`,
   `subs must be a whole number from 1 to 100000`; `stop_imaging_run` with `{"x":1}`,
   `stop_imaging_run takes no arguments`; `continue_imaging_run` with `{"anyway":true}`
   and no run, `no imaging run is running`; and on a console without
   `with_imaging_run_steps`, `start_imaging_run`, `stop_imaging_run` and
   `continue_imaging_run` each answer `this console runs no imaging runs` while
   `get_imaging_run` answers `{"run":null}`. `start_imaging_run` with
   `{"exposure_ms":1}` and `continue_imaging_run` with `{}` each answer a result with
   `isError` true. The test leaves 1 s of virtual time between two calls of
   `start_imaging_run`, so the rate limit of US10-S4 is not what answers.
4. **Given** the same console with `refuse(Part::Solver, "this console has no imaging solver")`,
   **When** a committed test calls `start_imaging_run` three times within one second of
   virtual time, **Then** it asserts that the first two answer the tool error
   `this console has no imaging solver`, the words `POST /api/imaging/run` answers with
   409 on the same console, and the third
   `the rate limit of 2 moving calls a second was reached`; and that the log holds three
   lines.
5. **Given** the same console with a run started over MCP that is stacking, after
   `set_stack(4, &[], false)` and a settle, **When** a committed test calls
   `stop_imaging_run`, **Then** it asserts a `run` whose `subs.stacked` is 4, a
   `run` with `state` `ended` and `ended.reason` `stopped`, and no new line in the call
   log. **Given** a run paused as in US6-S1, **When** the test calls
   `continue_imaging_run` with `{"anyway":true}`, **Then** it asserts a `run` whose
   state is no longer `focus_pause`.
6. **Given** `crate::support::mcp::motion::Rig` (`crates/finder/tests/support/mcp.rs:182`)
   after a sync, its state given `with_imaging(R)` and
   `with_imaging_run_steps(script.steps())` and served again through the door, **When**
   a committed test calls `move_axis` for the azimuth axis at 0.5°/s for 30 s and then
   `start_imaging_run`, **Then** it asserts the tool error
   `a direct axis move is running` and an empty `calls()`; and that after `stop` and a
   wait of 3 s `start_imaging_run` is accepted.
7. **Given** the scripted console, **When** a committed test calls each of the four
   tools once with valid arguments, **Then** it asserts that no answer, and no line of
   the call log, contains `test-token`.

---

### User Story 11 - The README and the field checklist (Priority: P3)

As the owner on the first night with the whole chain, I follow a written checklist from
"camera on the focuser" to "a saved stack", started once from the page and once by an
MCP client, and I know what each state and each ending means.

**Why this priority**: The run cannot be proven at a desk; the checklist is how the
field checks are carried out and written down.

**Independent Test**: `crates/finder/tests/console/imaging_run_docs.rs`, the module
`imaging_run_docs` of `console`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console imaging_run_docs::`.

**Acceptance Scenarios**:

1. **Given** `crates/finder/README.md` read by the test from
   `CARGO_MANIFEST_DIR`, **Then** a committed test asserts that it holds the heading
   `## An imaging run`, and that the text from that heading to the next `## ` heading
   holds, each between backticks, every word of `finder::imaging::run::STATES` and every
   word of `finder::imaging::run::ENDINGS`, the three routes, the four tool names, and
   `run.json`.
2. **Given** the same section, **Then** a committed test asserts that it holds the
   heading `### Field checklist: the first imaging run`, and that the text from that
   heading to the next heading holds exactly fourteen lines that begin with a number
   and a full stop, numbered 1 to 14 in order, and names `Start imaging run`,
   `start_imaging_run`, `Stop` and `run.json`; and that the text from the line that
   begins `8.` to the line that begins `9.` contains the word `corner` (the known limit
   of the run's own imaging frames, Clarifications).
3. **Given** the same file, **Then** a committed test asserts that its `## API` table
   holds a row for each of `POST /api/imaging/run`, `DELETE /api/imaging/run` and
   `POST /api/imaging/run/continue`, and that the text of `## MCP clients` names the
   four tools.

---

### Edge Cases

- **The console exits mid-run.** Nothing resumes at the next start: a run is run-only,
  like Track. A run that was stacking leaves the `run.json` of its last change of state
  with no `ended`; one that had not yet stacked leaves none. The scope node's watchdog
  stops the mount when the console is gone, as for any motion.
- **The observer turns Track off, or presses Stop, during a run.** Both are the
  console's Stop; the run ends `stopped` (US4-S3).
- **An MCP client calls `set_tracking` or `goto_target` during a run.** `set_tracking`
  on for the same target changes nothing; off is Stop. `goto_target` is refused with
  `Track is on`, as it is today (`crates/finder/src/api.rs:613`).
- **Someone asks for an imaging frame, the focus aid, a set of darks or flats, or a
  stack during a run.** Each is refused. The lease refuses with the name of whoever
  holds the camera then, in 031 US9's words `the imaging camera is held by` and the
  name: the imaging run, the focus aid or the stack (US4-S4, US6-S7). The starts of
  033, 034 and 035, as drafted, look for an imaging frame in flight only while nobody
  holds the lease, so they too answer with the holder's name while the holder is
  exposing, and never `an exposure is in progress`.
- **Another taker gets the lease in the instant of a hand-over.** The run's next step
  is refused with that holder's name. A focus check that is refused pauses the run, as
  one that could not finish; a stack that is refused ends it `stack_failed` (US2-S4); a
  lease that cannot be taken back after the focus gate ends it `camera_failed`
  (US1-S2).
- **A run is asked for while one of those holds the camera.** The start is refused with
  the lease's words, which name the holder, before Track is asked, even while the
  holder's own imaging frame is exposing: the camera part asks for the lease before it
  looks for an imaging frame in flight (FR-021, US6-S7).
- **A run is asked for while a single imaging frame, taken under no lease, is in
  flight.** The camera part takes the lease, sees the imaging frame, gives the lease
  back and refuses with `an exposure is in progress`.
- **The stack is stopped from its own route or tool during a run.** The run sees a stack
  that no longer runs and ends `stopped` (US7-S4).
- **Stop lands while the run is re-centring.** The run asks for Stop once more and ends
  `stopped`, never `mount_failed`, whatever Track answered meanwhile; the mount stays
  at rest (FR-020, US5-S5).
- **The run's own stop arrives while it is re-centring.** The re-centre is finished
  first, within about 5 s, and the run then ends `stopped` with Track on, as after any
  stop of the run (FR-005, US5-S3; owner decision 14).
- **The console is dropped in the middle of a run** (a test does this; the binary's
  state lives until it exits). The runner's task is aborted, nothing more is asked of
  any part, and Track is let go as 015 has it (FR-007, US1-S6).
- **The aim point was tapped, or measured on an earlier night.** The first imaging solve
  replaces or refines it (032). Centring starts from whatever Track brought to the aim
  point in force; with a good aim point it needs no try (US4-S1).
- **The target is outside the imaging frame at the first solve.** The offset is the
  separation of the solve's centre from the target, which needs no pixel of the target;
  a wide first search is 032's.
- **An imaging solve that does not move the aim point**: 032 took no aim reading from it
  (`moving`, `no_live_pointing`, `off_frame`, `set_aside`), or took one whose median
  lies within 10″ of the aim point in force and left that alone, and 032 holds no
  measured aim point back. A re-centre would bring the same aim point back, so the run
  takes another imaging frame instead; it still counts as a try (US5-S3). An aim point
  032 holds back while Track is on, which is where every aim reading of a run goes, does
  move it: the re-centre's Track off puts it in force (FR-022; 032's FR-036).
- **No finder live pointing arrives after the re-centre's Track off.** After 5 s the
  run asks Track anyway, and Track decides with the pointing it has, taken at the old
  aim point; an aim point 032 held back stays held under the new Track, so the next
  solve asks for another re-centre, and at worst a try is spent. A finder camera that
  gives a live pointing less often than every 5 s is already outside what 015's tracker
  is built for (`crates/finder/src/tracking.rs:248`).
- **A newly measured aim point while Track is on.** 032 holds it back until Track is
  off, so the tracker never chases it and, since 038, it is never read as a push by
  hand that marks the pointing model's slip (032's FR-036; 032's owner decision 25, which
  owner decision 26 cites, settled on 2026-11-17). The re-centre's Track off puts it in
  force, and the live pointing the re-centre then waits for is the first taken at it
  (FR-025). As 032 was first drafted, the tracker chased it for up to the 5 s of a
  wait made with Track on (Clarifications).
- **Track is still stopping when the re-centre asks it again.** GoTo refuses with
  `the mount is already moving` while an axis still reports motion
  (`crates/finder/src/goto.rs:739`), and the run ends `mount_failed` with those words.
  Whether the simulator or the real mount can be that slow to stop is not established
  (plan.md § Risks, § Prerequisites 5).
- **The tracker is never steady.** The run stays in `tracking`, or in
  `paused_by_tracker` once it stacks, until the observer stops it, a guard ends the
  motion or, while it stacks, the time limit is reached. The run has no timeout of its
  own for an unsteady tracker: on a later mount a meridian flip is a long not-steady
  stretch (rule 1). The time limit is another matter: it counts the clock from the
  start of stacking, pauses included, because its job is to end a forgotten run
  (FR-030, US7-S2).
- **Slightly soft focus.** The run goes on and keeps the warning in `run.warnings` and
  `run.json` (US2-S3).
- **A focus pause while tracking.** Track stays on through the pause, so the target
  stays in the focus aid's view; Track's guards can still end the run (US6-S5).
- **The focus check cannot finish** (no tile of the imaging frame had five stars, the
  camera failed under it, someone stopped it from its own route). The run pauses as for
  unmeasurable and says why in a warning (US6-S1).
- **A run started while Track is already on the target.** Track answers at once and the
  run goes straight to its first imaging frame.
- **Two runs in one second on one target that both end before stacking.** The second
  folder's name ends `-2` (US3-S6).
- **No matching master dark or flat.** The run does not look: 035's stack says what is
  missing in its own `stack` object, beside `run` in the same view.
- **A camera whose exposure is minutes long** (a later equatorial mount). Nothing here
  waits a fixed time for an imaging frame; a stop abandons the run's own exposure with
  031's abort (FR-005).
- **An exposure setting longer than the tracker's limit, on an alt-az mount.** The
  stack's subs are capped by 036; the run's own imaging frames are not, so their
  corners smear and their centre does not once the target is centred. The solve's
  centre is all the run reads; a solve that fails for lack of stars is counted
  (US2-S2). A known limit (Clarifications), which the field checklist has the observer
  look for (FR-041).
- **The kept subs fill the disk.** The first failed save ends the run `save_failed`; the
  default time limit of owner decision 8 is what keeps a forgotten run from getting
  there.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The console SHALL carry out an imaging run through five parts it is handed,
  each a trait in `finder::imaging::run`: the mount, the camera, the focus check, the
  solver and the stack. The run SHALL reach the mount through the mount part only,
  whose three calls are: say the target, ask Track for it, and ask Track for it again.
  Neither `crates/finder/src/imaging/run.rs` nor any file under
  `crates/finder/src/imaging/run/` SHALL send an axis command, read an axis position,
  compute an altitude, an azimuth, a field rotation or a rate, or hold a number sized
  to one telescope (a focal length, a field size or a scale).
- **FR-002**: A start SHALL be refused, in this order and with nothing asked of a later
  part: with 400 for a `subs` outside 1 to 100000 or a `duration_s` outside 1 to 43200;
  with 409 and `an imaging run is already running` while one is on; with 409 and the
  part's own words when the camera or the solver says it is not ready, in that order;
  with 409 and the mount part's words when there is no target; with 409 and the camera
  part's words when it cannot take the lease of the imaging camera; and with 409 and
  Track's own words when Track refuses, in which case the lease SHALL be given back
  first. A refused start SHALL leave the earlier run's view, or none, in place. A
  started run SHALL give the lease up just before each focus check and just before it
  starts the stack, SHALL take it again when it goes on from the focus gate to the
  solve, and SHALL end `camera_failed` with the camera part's words when it cannot.
- **FR-003**: An accepted start SHALL go through these states, each shown by its word:
  `slewing` while the mount part says a GoTo is running; `tracking` while it tracks but
  is not steady; `first_frame` while the run's first imaging frame is taken;
  `focus_check` while the focus part checks focus; `solving` while an imaging frame is
  taken and solved; and `stacking` once the stack is asked to start. The run SHALL
  start a take of its own, or a focus check, only while the mount is tracking and
  steady; it does not watch the tracker during the exposures of a focus check or of
  the focus aid. `STATES` SHALL list the ten state words and `ENDINGS` the seventeen
  ending reasons.
- **FR-004**: The run's view SHALL hold `id`, `state`, `by` (`page` or `mcp`), `target`
  (`id`, `name`, `ra_deg`, `dec_deg`), `started_at` (console Unix seconds), `elapsed_s`
  (rising while the run is on, fixed once it has ended), `settings` (`subs` or null,
  `duration_s`, `keep_subs`), `subs` (`stacked` and `dropped` by reason), `focus`
  (`verdict`, `reading`, `record`, `unit`) and `centring` (`tries`, `offset_arcsec`,
  `tolerance_arcsec`) once known, `message`, `warnings` (left out while empty), `ended`
  (`reason`, `at`, `message`) once ended, and `folder` once the run has one. The `id`
  SHALL be the start's UTC as `YYYY-MM-DDTHH-MM-SSZ`, a `-`, and the target's name with
  every character that is not an ASCII letter or digit made `-`, by 035's one function
  for that rule, which `run::label` calls. Every message SHALL be worded as plan.md
  § The run's words gives it.
- **FR-005**: `Runner::stop` SHALL end a run that is on as `stopped`, whatever its state,
  and answer once it has ended; with none on it SHALL answer the latest view. Every wait
  of the run SHALL give way to a stop and to an ending of the mount's motion within one
  poll of 250 ms; a take that is given up SHALL be aborted through the camera part, and
  a focus check that is given up SHALL be stopped through the focus part. Five calls
  are the exception, because abandoning one half-way would leave Track off, or a stack
  or a focus aid running, with nobody owning it: starting and stopping the stack,
  opening and closing the focus aid, and asking Track again. The run SHALL wait for
  each to answer and act on a stop as soon as it has. Each is bounded by the part
  behind it; asking Track again takes at most the 5 s of FR-025, one Track off and one
  Track on.
- **FR-006**: When the mount part reports that its motion ended, the run SHALL end with
  the reason it gives: `stopped`, `too_low`, `sun_up`, `link_lost`, `target_changed`,
  `pushed_away`, or `mount_failed` with the mount's words. At every ending the run SHALL stop a focus
  check it gave up, stop the stack if it started it, which saves it, and give the lease
  back whether or not it still holds it, in that order, and SHALL ask nothing of the
  mount (owner decision 1).
- **FR-007**: A `Runner` that is dropped SHALL abort its run's task, and the task SHALL
  hold nothing that keeps the `Runner` alive. An aborted task SHALL ask nothing more of
  any part and SHALL let go of every part it holds, so that a console dropped in the
  middle of a run lets go of Track as it does without one.
- **FR-008**: `finder::imaging::run::script` SHALL offer `Script`, a stand-in for all
  five parts with the default answers, controls, holds and call list of plan.md § The
  script. Its stack SHALL run from an accepted start until it is stopped or ended by
  `end_stack`, whatever counts `set_stack` gives it (owner approval 5).
- **FR-009**: A take that the camera part fails SHALL be asked again; three failures in a
  row SHALL end the run `camera_failed` with the last error, and a failure to save SHALL
  end it `save_failed` at once.
- **FR-010**: A solve that finds no solution SHALL be followed by a new imaging frame and
  another solve, without another focus check; three in a row SHALL end the run
  `solver_failing`, and a solver that cannot run SHALL end it `solver_unavailable` at
  once.
- **FR-011**: While stacking the view's `subs` SHALL be the stack part's counts as last
  read, and the state SHALL read `paused_by_tracker` while the stack says it is waiting
  for the tracker. A focus verdict of slightly soft SHALL add a warning and go on.
- **FR-012**: A stack that cannot start SHALL end the run `camera_failed`, `save_failed`,
  `solver_unavailable` or `solver_failing` by the failure's kind, and `stack_failed`
  otherwise, without the stack being stopped. A stack that does not end cleanly at an
  ending SHALL add a warning and leave the ending's reason as it is.
- **FR-013**: `POST /api/imaging/run` SHALL accept a JSON object with the optional keys
  `subs`, `duration_s` and `keep_subs` and no other, under the same-origin rule, and
  start a run by `page` through `start_motion` with the tool name `imaging_run`. It
  SHALL answer 200 with the imaging view; 403 for a foreign origin; 415 for a body that
  is not JSON; 400 for JSON of another shape or FR-002's values; 409 with
  `this console runs no imaging runs` on a console given no parts; and FR-002's 409s
  (owner approval 1).
- **FR-014**: `DELETE /api/imaging/run` SHALL, under the same-origin rule, end the run
  that is on as `stopped`, wait until it has ended, and answer 200 with the imaging
  view; with no run on it SHALL answer 200 and change nothing. It SHALL not ask for Stop
  (owner decision 14).
- **FR-015**: `POST /api/imaging/run/continue` SHALL accept exactly `{"anyway":true}` or
  `{"anyway":false}` under the same-origin rule and answer 200 with the imaging view,
  409 with `no imaging run is running` when none is on, and 409 with
  `the imaging run is not paused` when the run is not in `focus_pause`.
- **FR-016**: The imaging view SHALL carry `run` from the first accepted start until the
  console exits, in every state of the view, `no_link` and `error` included, and so
  SHALL `imaging` in `GET /api/session` and in the MCP tools that return the session.
  Nothing about a run SHALL be written to `session.json` (owner approval 2).
- **FR-017**: A run SHALL keep `run.json`: in the stack's folder from the moment the
  stack has started, and, for a run that ends before that, in `runs/<id>/` under the
  imaging folder, made at the ending, with `-2`, `-3` and so on added when the name is
  taken. It SHALL hold `format` `imaging-run/1`, `finder` (the console's version), `id`,
  `by`, `target`, `started_at`, `started_utc`, `settings`, `state`, `events` (one
  `{at, state}` for each state entered since the start, `ended` last), and `subs`,
  `focus`, `centring`, `warnings` and `ended` as the view has them. It SHALL be
  rewritten whole, under a `.part` name and renamed, at every change of state once the
  run has a folder, and at the ending. It SHALL hold no site, node address or token. A
  write that fails while the run is on SHALL end the run `save_failed`; one that fails
  at the ending SHALL add a warning (owner approval 4).
- **FR-018**: `AppState::with_imaging_run_steps` SHALL give the console the parts of its
  runs and attach the run's view to the imaging view, and `crates/finder/src/lib.rs`
  SHALL list the three routes.
- **FR-019**: The console's own mount part SHALL say the session's target, or refuse with
  `no target is selected`, and SHALL point by calling the function the Track route
  calls, passing its refusals on unchanged. It SHALL report `slewing` while Track is
  `armed`; tracking, with the tracker's own answer to "steady" (036), in every other
  state of Track but `off`; and an ending with the reason of plan.md § The console's
  mount part when Track is off.
- **FR-020**: The console's mount part SHALL point again by turning Track off with
  `Session::stop_tracking` and calling the function of FR-019 once more under
  `start_motion`. It SHALL remember the stop generation of `GoTo::stops` when it points,
  and report the ending `stopped` whenever that generation has changed. Pointing again
  SHALL be refused as stopped when it has changed beforehand. When it has changed by
  the time Track has answered, whatever Track answered, the part SHALL carry out the
  console's Stop once more and answer that it was stopped; only with the generation
  unchanged SHALL an error from Track be a failure of the mount. Track read `off` with
  the reason `stopped` SHALL be the ending `stopped`, except between the part's own
  Track off and Track's answer.
- **FR-021**: The console's own camera part SHALL refuse a start with 031's words while
  the imaging camera is not ready. It SHALL take 031's lease of the imaging camera
  (031 US9) as `the imaging run`, passing a refusal's words on unchanged, before it
  looks at anything else of the camera (owner approval 2). Holding the lease, it SHALL refuse with
  `an exposure is in progress` while an imaging frame is in flight, and with
  `the imaging camera has a region of interest set: clear it before an imaging run`
  while the settings hold one (owner decision 24), giving the lease back each time; and
  it SHALL give the lease back when asked. A take SHALL ask 031, as the holder of the
  lease, for one imaging frame with the purpose `run` and wait for it: `saved` answers
  its id and file; an error beginning `could not save the imaging frame` is a save
  failure; any other failure, a refused take and an aborted exposure are camera
  failures.
- **FR-022**: The console's own solver part SHALL refuse a start with
  `this console has no imaging solver` when 032 has none. A solve SHALL ask 032 to solve
  the imaging frame and wait for it. Solved, it SHALL answer the separation of the
  solve's centre from the target in arcseconds; the tolerance, the larger of 1/20 of the
  imaging frame's shorter side times the solve's scale and 90″ (owner decision 2); and
  whether the solve moved the aim point: whether, after the solve, the session's aim
  point is another than it was before it, or the session holds a measured aim point
  back while Track is on (032's FR-036), which the re-centre's Track off puts in force
  (owner decision 26). Failed with 032's reason `no_database`, or
  `failed`, it is a solver that cannot run, unless the error begins
  `could not save the imaging solve`, which is a save failure; with any other reason,
  no solution.
- **FR-023**: The console's mount part SHALL report the ending `target_changed` when the
  session's target is no longer the one the run started with (owner decision 12), and
  the ending `pushed_away` when the session's `track` view carries 038's `push` (owner
  decision 27).
- **FR-024**: After a solve the run SHALL keep its offset and tolerance in `centring`.
  Outside the tolerance it SHALL count a try, show `centring`, ask the mount part to
  point again when the solve moved the aim point as FR-022 tests it, wait until the
  mount tracks and is steady, take another imaging frame and solve it. After three tries
  without success it SHALL end `not_centred` without starting the stack (owner
  decisions 3 and 4).
- **FR-025**: After it turns Track off and before it asks Track again, the mount part's
  pointing again SHALL wait, with Track off, for a live pointing newer than the one it
  finds once Track is off: at most twenty polls of 250 ms, 5 s. That live pointing is
  the first the session records with Track off, which puts a measured aim point 032
  held back in force and is taken at it (032's FR-036, owner decision 26). It SHALL be
  refused as stopped when the stop generation changes during the wait, and SHALL go on
  to ask Track when no live pointing has come.
- **FR-026**: A focus verdict of clearly out, unmeasurable or no record, and a focus
  check that could not finish, SHALL put the run in `focus_pause`, open the focus aid
  through the focus part, and say so in the words of plan.md § The run's words (owner
  decision 21). A focus aid that cannot be opened SHALL add a warning and leave the run
  paused.
- **FR-027**: `Runner::resume` SHALL be refused outside `focus_pause`. In it, with
  `anyway` false the focus check SHALL be repeated, and with `anyway` true the run SHALL
  go on to the solve unchecked with the warning
  `Focus was not checked again: the observer went on anyway.` (owner decision 5).
- **FR-028**: A focus pause 900 s old SHALL end the run `focus_timeout`. Every way out of
  a pause, a resume or an ending, SHALL close the focus aid through the focus part
  first, whether or not the aid could be opened (owner decision 6).
- **FR-029**: The console's own focus part SHALL check focus with 033's focus check and
  answer its verdict, its focus reading and the focus record it was judged against, in
  the unit 033 shows; SHALL answer a check that ended any other way than with a verdict
  as one that could not finish, with 033's reason; and SHALL open and close 033's focus
  aid, which also stops a check that is running.
- **FR-030**: While stacking the run SHALL end `count_limit` when the subs stacked reach
  the request's `subs`, and `duration_limit` when the request's `duration_s`, 7200 s
  when the request gave none, have passed on the clock since the stack started. Time in
  which the tracker pauses the stack SHALL count; time before the stack started SHALL
  not (owner decision 8).
- **FR-031**: When ten subs in a row have been dropped with none stacked between them,
  the run SHALL end `subs_failing`, naming each reason that rose and by how much (owner
  decision 9).
- **FR-032**: A stack that reports a failure SHALL end the run `camera_failed`,
  `save_failed`, `solver_unavailable` or `solver_failing` by the failure's kind, and
  `stack_failed` otherwise; a stack that stops with none SHALL end it `stopped`. In
  each case the run SHALL still ask the stack to stop, which saves it.
- **FR-033**: The console's own stack part SHALL tell 036's steering that the set point
  is the sensor's centre and start 035's stack for the run's target, answering the
  stack's folder; SHALL answer its counts, with the reasons whose count is above 0,
  whether 036's steering is waiting for the tracker, and why it ended by itself; and
  SHALL stop it, which saves it. With `keep_subs` false and a saved stack it SHALL then
  remove each sub's file and solve sidecar that `stack.json` lists (owner decision 22).
- **FR-034**: `AppState::with_imaging_run` SHALL give the console its own five parts,
  and the `finder` binary SHALL call it after 032's imaging solver is set, under
  `--simulate` too.
- **FR-035**: The Imaging page SHALL show a block headed Imaging run, first on the page,
  whenever the imaging view is `ready` or holds a `run`. With no run on it SHALL offer
  `Start imaging run` with the two optional limits and `Keep every sub`. While one is on
  it SHALL show the run's message as a status (an alert in `focus_pause`), the counts,
  the elapsed time, `Started by an MCP client` when `by` is `mcp`, each warning, and
  `Stop imaging run`; in `focus_pause` also `Check focus again` and `Go anyway`. After
  an ending it SHALL say why it ended and what was saved, as a status for `stopped`,
  `count_limit` and `duration_limit` and as an alert for every other reason (owner
  approval 6).
- **FR-036**: `crates/finder/web/src/api.ts` SHALL give the run's type and the three
  calls; `DESIGN.md` SHALL describe the block under the heading
  `### Imaging run — built`; `.impeccable/design.json` SHALL list one component named
  `Imaging run`; and `crates/finder/web/dist` SHALL be the build of its source.
- **FR-037**: The console SHALL offer the MCP tools `start_imaging_run`,
  `stop_imaging_run`, `continue_imaging_run` and `get_imaging_run`, each passing through
  to the action its route calls, refusing with the route's words, and answering
  `{"run": …}` with the run's view or null (owner approval 3).
- **FR-038**: `start_imaging_run` SHALL be a moving tool: it SHALL take the rate limit
  first, start the run by `mcp` through `start_motion` with the tool's own name, be
  refused with `a direct axis move is running` while one runs, and write one call-log
  line. The other three SHALL take no rate limit and write no line (owner decision 19).
- **FR-039**: The `## API` table of `crates/finder/README.md` SHALL hold a row for each
  of the three routes, and its `## MCP clients` SHALL list the four tools (owner
  approval 7).
- **FR-040**: `crates/finder/README.md` SHALL gain `## An imaging run`, saying what a
  run does in order, each state and each ending by its word, the limits, the three
  routes and four tools, where `run.json` lies and what it holds, that the imaging
  camera is held throughout, by the run, the focus aid or the stack, that the run moves
  the mount only through Track and never stops it, what Stop does, and that the run's
  own imaging frames are taken at the camera's exposure setting while only the stack's
  subs are capped (owner approval 7).
- **FR-041**: That section SHALL hold `### Field checklist: the first imaging run`,
  fourteen numbered steps from the camera on the focuser to a saved stack, with the run
  started once from the page and once by an MCP client, and what to write down at each,
  whether the corner stars of the run's solved imaging frames are trailed among them.

### Key Entities

- **Imaging run**: the console's whole procedure from a chosen target to a saved stack
  (`GLOSSARY.md`).
- **Part**: one of the five things a run asks: the mount, the camera, the focus check,
  the solver, the stack.
- **State**: where a run is, one of ten words.
- **Ending**: why a run is over, one of seventeen reasons, with a message for the
  observer.
- **The run's record**: `run.json`, in the stack's folder or in `runs/<run id>/` under
  the imaging folder.
- **Lease**: 031's one hold on the imaging camera (031 US9), which a run takes for its
  own imaging frames and hands over to the focus check and the stack (the plan
  document, amendment 1).
- **Set point**: where on the sensor the run wants the target, today its centre (036's
  glossary term).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or `playwright.config.ts` is edited. The only edits to existing
  files under `crates/finder/tests` are the `mod` lines of owner approval 8.
- **SC-003**: No `Cargo.toml`, `Cargo.lock` or `deny.toml` changes, no scope-link message
  is added, and no `unsafe` is written.
- **SC-004**: On the simulator, a run started with no aim point reaches stacking with the
  target within the centring tolerance plus 20″ of the sensor's centre (US5-S4).
- **SC-005**: On the simulator, after Stop the mount is at rest and stays at rest,
  whatever the run was doing (US4-S3, US5-S5), and no ending but a guard's or Stop's
  changes what Track is doing (US8-S1, US8-S2).
- **SC-006**: No file of this spec under `crates/finder/src/imaging/` holds an
  identifier, an import or arithmetic for an axis, an altitude, an azimuth, a rate or
  axis counts, and none holds a focal length, a field size or a scale as a number. The
  rule is about code: the words an observer reads and the doc comments are exempt, so
  the `too_low` message may name the minimum altitude and a comment may say that
  nothing here knows an axis (FR-001; plan.md § Plan traps 18).
- **SC-007**: Every story's diff, measured as plan.md § Plan traps 3 says, is under
  64 KiB.

## Assumptions

- **Earlier features are in place.** 014, 015, 016 and 017 have landed; 030 to 036 have
  not (plan.md § Prerequisites). Every `path:line` here was read at 49bb210, and read
  again there after the audit. A file that 030 to 036 create is named without a line,
  with the spec that creates it.
- **Out of scope:**
  - an equatorial mount, meridian flips, dithering and any tracker but 015's as 036
    leaves it; the run is written so that none of them needs a change here;
  - plans of several targets, schedules, and a run that resumes after the console
    restarts;
  - autofocus, and any change to 033's verdict or its thresholds;
  - choosing the exposure, the gain or the binning: they are the imaging camera's
    settings (031), capped by 036 for a stack's subs only; capping the run's own
    imaging frames too, which needs a getter 036 does not offer (Clarifications);
  - any change to GoTo, Track, Stop or their guards, and any new mount command; what
    the tracker does with a newly measured aim point is 032's and 015's, and the owner
    settled it as 032's owner decision 25: 032 holds it back while Track is on (owner
    decision 26);
  - the lease itself, and how 033 to 035 take it: they are 031's and theirs;
  - a free-space check (owner decision 7);
  - a `target` argument (owner decision 10);
  - listing, fetching or deleting runs through the API, and returning the stacked
    picture from a run tool: the picture is 035's `get_stack`;
  - MCP on the scope node.
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
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007, FR-008]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-009, FR-010, FR-011, FR-012]
US3:
  depends_on: []
  depends_on_merged: [US1, US2]
  implements: [FR-013, FR-014, FR-015, FR-016, FR-017, FR-018]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-019, FR-020, FR-021, FR-022, FR-023]
US5:
  depends_on: []
  depends_on_merged: [US3, US4]
  implements: [FR-024, FR-025]
US6:
  depends_on: []
  depends_on_merged: [US5]
  implements: [FR-026, FR-027, FR-028, FR-029]
US7:
  depends_on: []
  depends_on_merged: [US6]
  implements: [FR-030, FR-031, FR-032]
US8:
  depends_on: []
  depends_on_merged: [US3, US5, US6, US7]
  implements: [FR-033, FR-034]
US9:
  depends_on: []
  depends_on_merged: [US8]
  implements: [FR-035, FR-036]
US10:
  depends_on: []
  depends_on_merged: [US8, US9]
  implements: [FR-037, FR-038]
US11:
  depends_on: []
  depends_on_merged: [US7, US10]
  implements: [FR-039, FR-040, FR-041]
```

Every edge is a merge edge, and the stories run one after another. An edge names each
earlier story of this spec that last wrote a file the story writes next, so that the
later story builds on the merged file.

- **US2** grows US1's machine in `crates/finder/src/imaging/run.rs`: the retries, the
  table of failed steps, the stack's counts and the warning for soft focus.
- **US3** serves that `Runner` and adds the record to its machine. It names US1 for
  `crates/finder/src/imaging/mod.rs` and US2 for `run.rs` and
  `crates/finder/tests/console/main.rs`.
- **US4** writes the console's own mount, camera and solver parts in
  `crates/finder/src/imaging/run/console.rs` and drives them through US3's routes; it
  adds one line to `run.rs`.
- **US5** adds the centring loop to the machine and the wait for a live pointing to the
  mount part of US4, and is the first story to ask for a re-centre. It names US4 for
  `run.rs`, `console.rs` and `crates/finder/tests/motion/main.rs`, and US3 for
  `crates/finder/tests/console/main.rs`.
- **US6** adds the pause to the machine and the focus part to `console.rs`, after US5,
  and uses US3's `continue` route and US4's camera part.
- **US7** adds the limits and the stack's own endings to the machine, after US6.
- **US8** adds the stack part, assembles the five parts and wires the binary. Its whole
  run needs every earlier story: it names US7 for the count limit its scenarios end on,
  US6 for `console.rs`, US5 for `crates/finder/tests/motion/main.rs` and US3 for
  `crates/finder/src/api.rs`.
- **US9** calls the three routes and reads every state and ending the earlier stories
  make.
- **US10** calls the same actions as the routes. It writes `crates/finder/src/api.rs`
  next after US8, and follows US9 so that one story is on the floor at a time.
- **US11** documents all of it and tests the document against US1's two word lists; it
  writes `crates/finder/tests/console/main.rs` next after US7, and follows US10 because
  its README names the four tools and three routes as they were built.
