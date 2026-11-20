---
state: draft
depends_on_landed: ["015-tracking", "030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "034-dark-and-flat-frames", "035-live-stack", "038-track-follows-a-push"]
# Drafted 2026-11-15 against `main` at 49bb210 (the drafting worktree's HEAD,
# 49bb210fe4860dd98ba24727074e7bd3cc0d7124) from
# docs/plans/2026-11-15-specs-030-037-imaging.md, by the grilling session of that day.
# A first drafter was cut off; a second finished the trio the same day against the plan
# document's "Mount-neutral by design" and "Amendments" sections and the reviewed drafts
# of 030, 031, 032 and 034. A second agent then reviewed the trio, and the review was applied
# the same day against 035's revised draft (its plan.md with the camera's lease, its
# tasks.md and its Refinement Record) and 033's (plan.md § Refinement Record).
# An audit of the seven trios the series then had, side by side, was then applied the
# same day
# (§ Clarifications, the audit's session), and every `path:line` in spec.md and plan.md
# was checked again against 49bb210fe4860dd98ba24727074e7bd3cc0d7124, the worktree's
# HEAD: none had moved.
# When drafted it was not probe-built: no story had been compiled or run, and the
# bounds of US3-S1, US3-S2, US4-S5, US6-S5 and US7-S1 came from arithmetic on a model of
# the loop, not from the code.
# It builds on code that 030, 031, 032, 033, 034 and 035 create and that does not exist yet.
# Every interface of theirs named here is taken from that draft's plan.md. The trio is
# re-anchored against `main` after they land and before it is flipped (plan.md
# § Prerequisites).
# It stays draft until the owner settles the owner decisions and flips it to `ready`,
# which gives the approvals listed under "Owner approvals" and dispatches it.
# Probe-built 2026-11-18 and 19 (round 1): all eight stories were built from this trio,
# tests first and with both gates, on probe builds of 030 to 035, and every bound held.
# Its 3 blocking and 44 clarity defects were folded in on 2026-11-19 with the owner's
# answers of that day ("Yes, all four"): 038 has landed, so `follow` ends the claim and
# 038 joins `depends_on_landed`; a gap after a lapse sends the zeroed rate; and US1-S6,
# US3-S12 (now US9-S12) and US4-S7 are new (§ Clarifications, the session of round 1
# probes; plan.md § Prerequisites 5 and § Refinement Record, "What probe round 1
# changed").
# Reanalysed 2026-11-19: six reviews of that folded draft found 78 more issues (AA1 to
# FF20), which came to fifteen choices for the owner, answered "Take all 15" the same
# day. They are folded in, in two passes: a push by hand, 2 s of settling and a stopped
# axis are times the tracker is not steady; the steering stands back from a stack that
# begins more than 3′ from its set point; the claim lapses after eight gaps and a lapse
# holds the finder camera's step only; no gap changes a rate by more than 0.1″/s; the
# exposure is decided from the hour ahead, with a default smear of 6 px; the page's
# sentences say why exposures are paused; the step share and the rate share are
# settings; and `set_steering` is an MCP tool. User Stories 3 and 6 were cut by tests
# into User Stories 9 and 10, and User Story 11 is the tool: eleven stories (§
# Clarifications, the session of the reanalysis; plan.md § Refinement Record, "What the
# reanalysis changed"). Nothing of that second fold has been built: a second probe round
# builds the changed stories on the 035 that landed before this spec is flipped.
---

# Feature Specification: Tracking Steered by the Subs

**Feature Branch**: `036-tracking-from-subs`

**Created**: 2026-11-15

**Status**: Draft

**Input**: Spec 015 keeps a target at the aim point with the finder camera's solves. That
is enough for an eyepiece and not for an imaging camera: a finder solve wanders by tens of
arcseconds, more than 015's 10″ deadband, and each wander becomes a rate change that
smears a sub. Spec 035 solves every sub, and an imaging solve places the target on the
sensor to a fraction of an arcsecond. While a stack runs, take the tracking error from
each sub's imaging solve in place of the finder camera's live pointing, change the
mount's rates only between exposures for as long as a stack holds the tracker,
whichever of the two steers it, cap the exposure at the tracker's limit for a
chosen smear, pause exposures while the tracker is not steady, and show the drift and the
error. 015 lists "guiding from the imaging camera" as out of scope; this spec is that
work.

## Clarifications

### Session 2026-11-15 (the interview's decisions, from the plan document)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, interface,
  pinned shape and rejected alternative. A story that would change a decision escalates
  rather than re-deciding it.
- Q: What steers the tracker during a stack? → A: The subs. "The finder gets the target
  there; during exposures the subs' imaging solves steer the tracker, and rate
  corrections change only between exposures"
  (`docs/plans/2026-11-15-specs-030-037-imaging.md`, § Settled decisions). The rule
  holds for as long as a stack holds the tracker, after the subs have gone quiet too
  (owner decision 6); the switch turned off is the declared exception (owner decision
  8), and since 2026-11-19 a stack that began more than 3′ from its set point is a
  second: the steering stands back from it and the tracker's own corrections run as 015
  has them (FR-014).
- Q: How does imaging reach the mount? → A: Through four actions (GoTo, Track on and
  off, Stop, and a tracking error handed to whatever tracker is running) and two answers
  (the longest exposure the tracker allows, and whether it is steady enough to expose).
  This spec defines that as the one interface between the stack and the tracker. No file
  under `crates/finder/src/imaging/` computes a parallactic angle, an axis rate or a
  field rotation, or reads an axis count (the plan document, § Mount-neutral by design,
  rules 1 and 5).
- Q: Where does the target belong on the sensor? → A: At the set point, which the run
  names and which today is the sensor's centre. A later tracker may move it between subs
  with no change to the stack (rule 7).
- Q: May anything be sized to one telescope or to subs of a few seconds? → A: No. The
  exposure is a setting bounded by the tracker's limit, and timeouts and windows are
  counted in subs (rules 3 and 6). The deadband is counted in the mount's own counts,
  which the tracker reads from the mount.
- Q: Why can the finder camera not supply the tracking error? → A: A finder solve
  scatters by about 0.3′ to 0.5′, which is 33 to 55 imaging pixels in the worked
  example, and 2 px of trail in a 5 s sub is a rate error of 0.22″/s
  (`docs/research/2026-11-15-imaging-camera.md` § 5. Relating two cameras; § 7. Field
  rotation, sub length and the drive).
- Q: How small a correction can the mount be asked for? → A: One motor count. The
  worked example's mount reports 1,980,000 counts a turn, 0.65″ a count and 1.19
  imaging pixels, and in slow motion its controller advances its reference position one
  count at a time; what the tube does in answer is unmeasured
  (`docs/research/2026-11-15-imaging-camera.md` § 7. Field rotation, sub length and the
  drive). A step smaller than a count cannot be commanded, so the deadband is two
  counts and the smallest step, half of it, is one (owner decision 5). Whether a pulse
  of one count moves the tube is a field check (§ Assumptions).
- Q: Which subs hand the tracker a tracking error? → A: The ones the stack accepts.
  "While a stack is running, each accepted sub's imaging solve gives the target's offset
  on the sensor; that replaces the finder's live pointing as the tracker's error signal"
  (the plan document, § 036-tracking-from-subs). A sub the stack drops hands over
  nothing (owner decision 15 names the alternative).
- Q: Does this spec take exposures or hold the imaging camera? → A: No. The stack takes
  every exposure and is the camera's holder; this spec adds no lock (the plan document,
  § Amendments, 1). It keeps nothing on disk either (§ Amendments, 3).
- Q: What must carry over to another telescope? → A: Everything under
  `crates/finder/src/imaging/`. A later telescope on an equatorial mount answers "no
  limit" to the first question and "steady" to the second, except during a meridian
  flip, with no change to the steering or the stack (the plan document, § Settled
  decisions, Another telescope later).

### Session 2026-11-15 (review of `main` at 49bb210)

- Q: What was found in the tracker? → A: Nine things, each read at 49bb210:
  1. Each second the tracker sends each axis its feed-forward rate plus a correction
     (`crates/finder/src/tracking.rs:755`, `crates/finder/src/tracking.rs:868`). The
     correction is a proportional rate that lasts until the next solve is due, and an
     integral, the tracking offset (`crates/finder/src/tracking.rs:857`).
  2. Both are set in one place, `Shared::pointing`
     (`crates/finder/src/tracking.rs:256`), from the finder camera's live pointing: an
     error of 10″ or more (`crates/finder/src/tracking.rs:31`,
     `crates/finder/src/tracking.rs:300`) sets a proportional rate of 0.6 of the axis
     error over the solve interval (`crates/finder/src/tracking.rs:29`). A 20″ wander
     seen 2 s after the last solve is a rate of 6″/s for 2 s.
  3. The live loop calls it after every live solve
     (`crates/finder/src/live.rs:216`), and records the same solve as a sync for the
     pointing model in a separate call (`crates/finder/src/live.rs:223`). So the
     tracker can stop taking rates from the finder camera without the pointing model
     losing a sync.
  4. `Shared::pointing` also watches the cable's wind, takes the keyhole's resume
     pointing, keeps the 60 s of error samples the readout shows and feeds the backlash
     measure (`crates/finder/src/tracking.rs:274`, `crates/finder/src/tracking.rs:275`,
     `crates/finder/src/tracking.rs:315`, `crates/finder/src/tracking.rs:321`). Those
     stay on the finder camera.
  5. Every rate is sent by the tracker's own task, which the guard ends before Stop is
     sent (`crates/finder/src/tracking.rs:885`). The scope node drops a move still
     queued when Stop arrives, but accepts one that arrives after it. So a rate sent
     from any other task could start an axis after a Stop. Every send this spec adds is
     made by that task (FR-010).
  6. A take-up and the keyhole pass run inside one turn of that task
     (`crates/finder/src/tracking.rs:697`, `crates/finder/src/tracking.rs:551`), and the
     first reversal of an axis whose backlash is not measured opens a measurement that
     lasts minutes (`crates/finder/src/tracking.rs:829`) and that 015 closes from the
     finder camera's pointings (`crates/finder/src/tracking.rs:646`). The plan
     document's rule 1 gives the first two as today's examples of a tracker that is
     not steady; owner decision 17 declares three further times: the measurement, and
     the moments before a tracker's first rates and after its end.
  7. The axis never reverses on a correction: a requested rate against the axis's
     direction is sent as zero (`crates/finder/src/tracking.rs:873`). A step made in a
     gap keeps that rule.
  8. The longest exposure 015 reports is `smear / (corner radius × field rotation)`,
     with no limit under 10⁻⁹ rad/s (`crates/finder/src/ephemeris.rs:105`), and the
     session asks it for 1 px on a sensor described by three numbers
     (`crates/finder/src/session.rs:167`). Nothing consumes it.
  9. A tracker's first rates are sent by its task's first turn, which runs only after
     `start` has answered (`crates/finder/src/tracking.rs:207`,
     `crates/finder/src/tracking.rs:755`). A `gap` asked for the moment `start` answers
     therefore comes before any rate. The tracker rig below waits for the first rates,
     and a gap that comes before them moves nothing and claims nothing (FR-010).
- Q: What was found in the simulator? → A: Two things.
  1. The simulator's truth for an instant takes the telescope's direction as it is when
     the question is asked, and the sky as it was at the instant
     (`crates/simulator/src/optics.rs:120`). On a still mount that is exact. On a moving
     one, the truth of a sub's mid-exposure, asked for after the sub has arrived, is off
     by the mount's motion since: about 15″ for every second on a mount that tracks.
     032's simulated solver asks exactly that question (032 plan.md § The simulated
     adapter), so on a tracking mount every simulated imaging solve would be wrong by
     more than this spec's whole error budget. US1 gives the simulator a memory of where
     the telescope pointed.
  2. The simulated mount turns smoothly (`crates/simulator/src/mount.rs:97`), and it
     already takes a rate error (`crates/simulator/src/mount.rs:302`). A real
     controller's reference position advances one count at a time
     (`docs/research/2026-11-15-imaging-camera.md` § 7. Field rotation, sub length and
     the drive); what the tube does in answer is unmeasured. US1 adds a switch that
     makes the simulated tube follow whole counts, off by default: a model of that
     reference, not of a measured tube.
- Q: The plan document as first written had 036 add "rate error within an exposure" to
  the simulator. Where is it? → A: It is already there, in two parts this spec adds
  nothing to: the simulated mount's `set_rate_error` (`crates/simulator/src/mount.rs:302`), and 030's
  simulated imaging camera, which draws each star along the line between its places at
  the start and the end of the exposure (030 plan.md § The simulated imaging camera).
  No correction is made within an exposure, so a rate error within one is a straight
  trail, and US1-S1 asserts that trail on a mount that runs fast. What was missing is
  the truth of a past instant on a moving mount; US1 adds that, and the count steps,
  which is what the plan document's § Interfaces shared across the specs, Simulator,
  now names for 036.
- Q: Does a simulated sub show a drift? → A: Yes, once US1 lands. 030's simulated
  imaging camera draws each star along the line between its places at the start and the
  end of the exposure (030 plan.md § The simulated imaging camera), so a mount that
  drifts draws elongated stars, and with US1 the simulated solve's centre moves from sub
  to sub by the drift (US1-S1).
- Q: Why not hand the tracker the imaging error through its existing entry point? → A:
  Three reasons. Its proportional term is a rate held until the next solve, which is a
  deliberate drift during the next exposure; its integral winds up while a large first
  offset is being taken out; and its deadband is 10″. Owner decision 4 gives the control
  law used instead.
- Q: Is the simulator's refraction a problem? → A: No, and a test must know why. The
  tracker follows the target's apparent place; the simulator's optics are geometric. On
  the tracker rig the mount starts at the apparent place, so the simulated imaging
  centre sits about 65″ from M42's catalogue place, along altitude. The scenarios that
  play the stack themselves measure against the apparent place, as 015's tests do
  (`crates/finder/tests/motion/tracking/pi_loop.rs:217`). The scenarios that use the
  simulated imaging truth (US4-S5, US7) start from a mount pointed at the catalogue
  place: a start 65″ too high would need the altitude axis to back up, and no
  correction reverses an axis (item 7 above).
- Q: Does one tracker serve a whole night? → A: No. The session makes a new `Tracker`
  each time Track begins (`crates/finder/src/session.rs:914`) and stops the old one
  first (`crates/finder/src/session.rs:837`), and dropping a `Tracker` stops the mount
  (`crates/finder/src/tracking.rs:112`). So the steering never owns a tracker: it keeps
  weak references, and a tracker that has gone counts as one that was not steady.

### Session 2026-11-15 (the drafts of 030 to 035 and 037, as they stood when the review was applied)

- Q: How does 035's stack take and handle subs? → A: In two tasks (035 plan.md § The
  stack on the console). **The taker** calls `lease.take("sub")` back to back, on the
  camera's lease the stack holds as `the stack` from its start to its end, and never
  waits for the worker. It owns that lease, which drops when it returns, and it looks
  at the stack's stopping mark, one `tokio::sync::watch` of `bool`, before every take.
  **The worker** takes waiting subs oldest first: it builds the
  `SolveFrame`, runs the settings test, solves with `solving::solve_frame`, runs the
  focus gate, places the sub, adds it, and counts it at its step 8, "the one place a
  sub is accepted". Up to four subs wait between the two.
- Q: What is the camera's lease, and what does it mean here? → A: The plan document's
  amendment 1: one holder of the camera at a time, named, on 031's imaging state. 031's
  draft carries it as its US9 (031 plan.md § The camera lease), and 033 and 035 cite it
  name for name (035 plan.md § The camera's lease): `Imaging::lease`, and `Lease::take` and
  `Lease::set` for the holder. While a lease is out, `Imaging::take` and `Imaging::set`
  by anyone else are refused with the holder's name, `PUT /api/imaging/settings`
  included. So only the stack can change the exposure while it runs, and it does so
  through its own lease (FR-023). This spec takes no lease and adds no lock.
- Q: So when does a sub's tracking error reach the tracker? → A: Late. Sub N is solved
  while sub N + 1 is exposing, so its tracking error is applied in the gap before sub
  N + 2 at the earliest, and later when the worker is behind. A sub's tracking error is
  therefore from before the corrections of the gaps since its mid-exposure. The tracker
  keeps what each gap did, with its time, and allows for it (US3-S9, US3-S10); without
  that, every error would be corrected twice.
- Q: Can a stack change its exposure from one sub to the next? → A: No. 035 drops a sub
  whose `exposure_ms` differs from the reference sub's by more than 1 ms as `settings`
  (035 plan.md § The settings test), and 034 matches a master dark within 1 ms. And a
  take exposes for the console's `exposure_ms` setting, nothing else (031 plan.md § The
  console's imaging state); 035 says that "a sub taken at an exposure that changes
  within a stack is not offered" (035 plan.md § Prerequisites 10). So the cap is applied once,
  before a stack's first sub, by changing that setting with `Lease::set` on the stack's
  own lease, and the observer's value is put back through the same lease where the
  taker returns, before the lease drops with it, as 035's seam for 036 says (035
  plan.md § Shapes that existing tests pin; owner decision 2). 034's
  `Calibration::restore_exposure`, which called `Imaging::set` and so would be refused
  by the stack's lease, is not used: 034 has removed it (034 A1), as 035 notes (035
  plan.md § Shapes that existing tests pin).
- Q: How does the stack count a dropped sub? → A: Under one of eleven keys of
  `Dropped`, "every key always written" (035 plan.md § The stack on the console). This
  spec adds a twelfth, `not_steady` (owner approval 6). 035's tests sum whatever keys
  `dropped` holds and never assert how many there are, "since a later spec adds
  reasons" (035 spec.md § User Scenarios).
- Q: What does 037 read from this spec? → A: Two things when this was written, and more
  since 037's own fold (the session of the reanalysis, below, lists them all) (037
  plan.md § Prerequisites 2): whether the stack is waiting for the tracker, which is
  `steering.waiting` in the imaging view and `Steering::waiting()`; and whether the
  tracker is steady enough to expose, which is `Steered::steady()` of
  `Session::steered()`. It also makes one call:
  a run names the sensor's centre as the set point with `Steering::set_set_point(None)`
  as it starts its stack (037 plan.md § What 030 to 036 offer).
- Q: Which other specs write the same files? → A: 030 (the simulator), 031
  (`imaging/mod.rs`, `imaging/routes.rs`, `lib.rs`, the README, the Imaging page), 032
  (`imaging/solving.rs`, `session.rs`, and the files 031 names), 033, 034 and 035
  (`imaging/mod.rs`, `imaging/routes.rs`, `lib.rs`, the README, the page, the
  simulator's `optics.rs`), and 035 alone `imaging/stack/service.rs` and
  `StackBlock.tsx`. All six are in this spec's `depends_on_landed` and land first; 037
  lands after this spec and names it (037 spec.md, frontmatter). 035's trio was read
  whole when the review of this one was applied: its plan.md, its tasks.md and its
  Refinement Record. plan.md § What 035 must provide lists everything of it this spec
  uses or changes; 035's own Prerequisites 10 and 037's plan.md point at that section.

### Session 2026-11-15 (an audit of the seven trios the series then had, side by side)

- Q: What did the audit find here, and what was done? → A: Five findings, each checked
  against this trio, the plan document and the drafts of 030 to 035 and 037:
  1. **A1 (major). Rates changed inside exposures after the subs lapsed.** As drafted,
     once three gaps brought no tracking error the tracker's own corrections changed
     its rates as 015 has them, inside the exposures the stack went on taking, against
     the settled row "rate corrections change only between exposures". Applied, as
     close to the row as the design allows: while a stack holds the tracker and the
     subs have lapsed, `Shared::pointing` holds the step and the change of the tracking
     offset it would have made, and the next gap applies them (owner decision 6, FR-010,
     FR-011, US3-S6 [its lapse is now US9-S4], US3-S11); with no stack, 015's behaviour
     is unchanged. Owner approval 2 now covers every state in which gaps are served.
     While the tracker is not steady its own corrections run as 015 has them even after
     a lapse, because gaps are turned away then and 015 closes a first reversal's
     measurement from them (FR-021, US6-S5). The switch turned off is written as the
     observer's declared exception (owner decision 8).
  2. **A2 (minor). The stopping mark has a name.** 035's plan.md names it: one
     `tokio::sync::watch` of `bool`, which the taker races as
     `stopping.wait_for(|stopping| *stopping)` in a `tokio::select!` that is `biased`
     toward it. plan.md § The stack and § What 035 must provide now name it. The same
     reading of 035's plan.md showed that its taker owns the camera's lease and drops
     it as it returns, so the observer's exposure is put back there, by the taker, and
     not in 035's ending after the taker has returned (FR-023).
  3. **A3 (minor). Rule 1 is not edited.** Its two times are given as today's examples.
     Owner decision 17 alone declares the three further times, and plan.md
     § Prerequisites 9 no longer asks for the rule to change.
  4. **A4 (minor). The guard and the 5%.** FR-005 states the rule for every file under
     `crates/finder/src/imaging/`, while US2-S6 reads only `steering.rs` and US7 edits
     035's `stack/service.rs`. US7-S6 now reads that file for the same six names
     [thirteen since 2026-11-19] (FR-023). Owner decision 1's "about 5%" was in no
     source; it is now worked there from the research note's in-focus star size.
  5. **A5 (minor). A bound on the rate one tracking error may ask for.** No story
     changes. 037's owner decision 26 now cites 032's owner decision on a newly
     measured aim point while Track is on and no longer says its answer is written into
     036. If the owner takes, there, the answer that the tracker bounds that rate, US3
     carries the bound in `crates/finder/src/tracking.rs` with one scenario and owner
     approval 2 names it, before this trio is flipped (plan.md § Prerequisites 11).
- Q: Were the citations checked again? → A: Yes. Every `path:line` in spec.md and
  plan.md was read again at 49bb210, the worktree's HEAD. None had moved: the test files
  that changed between dbc2b5b and 49bb210 were already read at 49bb210 when the trio
  was finished.

### Session 2026-11-17 (the owner's answers)

- Q: What did the owner answer on a newly measured aim point while Track is on, and
  what changed here? → A: Hold it: 032 holds the aim point back while Track is on and
  uses it once Track is off (032's owner decision 25, which 037's owner decision 26
  cites). That is not the answer that would have had US3 bound the rate one tracking
  error may ask for (A5 above), so no story, scenario or requirement changes here;
  plan.md § Prerequisites 11 is met, and its Refinement Record and tasks.md say so.

### Session 2026-11-19 (round 1 probes)

Probe round 1 built all eight stories from this trio on 2026-11-18 and 19, tests first
and with both gates, in four agents, each starting where the one before stopped: US1 and
US2, US3 and US4, US5 and US6, and US7 and US8. The base held 030 to 034 as landed or as
their probes built them and 035's ten stories as its own round 1 built them. Every story
built as written and passed its gates under 64 KiB, and every one but US8 wrote only its
declared files (US8 needed one probe-only edit of a 034 test as that base carried it:
plan.md § Prerequisites 3). Every bound held, on three runs each. The round found 3
blocking and 44 clarity defects, D1 to D47, and all were folded into the three files on
2026-11-19 but for the points the last answer below lists (plan.md § Prerequisites 5
says what the round settled, and § Refinement Record, "What probe round 1 changed", what
it changed). Every case the round proposed where a mutation survived a story's committed
tests is now in its scenario, in the probe's words; each was run on the probe's build,
passed on the code and failed under the mutation. Where two fixes touched one sentence
they were merged.

- Q: What did the owner answer before the fold? → A: "Yes, all four", on 2026-11-19:
  1. Fold all 47 defects with their fix text, and every proposed case in its scenario in
     the reports' words.
  2. D25, a design gap. From US6 a pointing while the tracker is not steady runs 015's
     whole block even after a lapse and sets its proportional rate, and the first gap
     once the tracker is steady again zeroed that rate in the tracker's state but sent
     nothing, so the first exposure after a time not steady began on the stale rate:
     +0.632″/s on the azimuth axis for 0.15 s measured, up to 1 s and 0.6″ in general,
     against a sub's bound of 0.15″. That gap now sends the zeroed rate when 015's
     proportional term is still in force (FR-010; plan.md § The tracker, `serve_gaps`
     steps 2 and 4), with a case of US6-S1 [now US10-S1]. (The reanalysis completed this
     on 2026-11-19: every gap served while the claim is lapsed sends each axis its rate,
     since the tracker's own state cannot say what the mount was last sent; EE1.)
  3. D6, 038 has landed (`ad09efeb`), so plan.md § Prerequisites 12 is met as it said:
     FR-011 gains "`follow`, as `release` does, ends the claim and forgets what was
     pending or held", US3 gains US3-S12 [now US9-S12], and `038-track-follows-a-push`
     joins `depends_on_landed` and US3's shared write of the tracker.
  4. D8, the trio's own owner trigger in plan.md § Prerequisites 3 now reads "compares
     the whole object with a literal or a fixed list of keys": a comparison of two
     serializations of the same `Dropped`, as 035's `stack_console.rs` makes between
     `stack.json` and the view, is not one.
- Q: What could not pass as written? → A: Three things, now fixed. US6-S1's "the asker's
  `gap` answered after L": the turn that ends the take-up and the gap it then serves
  share L's instant in paused time, so it reads "at L or after it" (D24). US7's drift
  oracle read 031's `frame` part every 100 ms, but 035's taker takes the next sub at the
  instant one is saved, so the reads saw 3 subs of 18; a saved sub is now one whose FITS
  file is in the stack's folder, and its drift is read from that file's `DATE-OBS` and
  `EXPTIME` (D37). And a stack started at the instant of the tracker's first rates
  dropped its first sub as `not_steady`, since `steady_between`'s bounds are inclusive;
  the stacking rig now waits 1 s more before a stack starts (D38). (The reanalysis took
  that wait out again on 2026-11-19: the tracker settles for 2 s after its first rates,
  the stack waits that out by itself, and a sub that starts at the instant the tracker
  became steady is kept.)
- Q: What did the round measure that the trio now relies on? → A: On three runs each,
  with the same numbers each time: US3-S1's drift from the 12th sub at most 0.0220″ with
  each error handed over at once and 0.0340″ played late, against 0.15″ (4.4×; the
  model's 0.030″ was 13% under the late figure); US3-S2's median drift without feedback
  17.19″, against "above 3.0″" (5.7×); US4-S5's drift from the 14th sub at most 0.01552″
  (9.7×); US6-S5's worst true error 41.414″ against 015's 60″ (1.45×), with the play
  measured at 0.299879°, while with the held mode testing `lapsed` alone 015's bound
  broke at transit + 319.7 s, as plan.md § Plan traps 28 says; and US7-S1's drift from
  the 14th sub at most 0.01558″ (9.6×), and 0.746″ on every sub with the switch off.
  US5-S6's limit is 37.28 s on the rig, not the worked 37.15 s, which is corrected
  (D32). US3's test file passed unedited before and after US6. US7's stacking rig tracks
  and stacks, 18 subs in about 97 s of virtual time, and US7-S5's rig saw its link
  dropped within 1 s, well inside its gap. A first reversal's measurement held the
  tracker not steady for about 312 s of the simulator's time with 0.3° of play, and a
  take-up for 0.85 s. Each story's diff is under 64 KiB, US3 the largest at 50,893 B
  (77.7%), about 57,000 B (87%) with its folded cases and US3-S12 [now US9-S12] (plan.md
  § Risks).
- Q: Where do 015 and 030 to 035 differ from what this trio says? → A: 015's tracker on
  `main` carries 038's `follow`, which moved most of this trio's citations of it from
  line 83 on; the re-anchoring corrects them (plan.md § Prerequisites 2). The nine
  points of plan.md § What 035 must provide are all provided by 035's probe code, with
  three words the plan now gives: the taker's receiver of the stopping mark is made
  `mut` (D39); the `not_steady` drop is counted inline, as the settings drop is, since
  `Dropped::count` sends a key it does not name to `unprocessed` (D40: true of the
  probe's 035 and not of the 035 that landed, where the settings drop is answered from
  the worker's `process` and counted through `Dropped::count`, whose last arm is
  `unreachable!`; plan.md § The stack now gives US7's drop that way, with an arm for
  `not_steady`: reanalysis BB1); and the Stack
  block's filter reads an optional count with `?? 0` (D41). Since round 1, 035 has been
  split: its US5-S6, which this trio cited, is now US11-S3, in `stack_camera.rs` with
  the scripted camera, and this trio cites it so (plan.md § Technical Context names the
  module each of 035's rigs lives in).
- Q: Which scenarios are new, and which alternatives were taken? → A: Three scenarios
  are new: US1-S6, the memory's hour (D2), placed last so that US1-S5 keeps its number;
  US3-S12 [now US9-S12], a followed push (D6); and US4-S7, the view's `steering` part
  and `Imaging::steering` (D20). Of the reports' alternatives: the gap at which the
  claim lapses still sends nothing, and FR-010 now says so (D11); FR-021 keeps the list
  of 64 and says what it means for a window before the start (D33); `Dropped::total()`
  keeps its line in the plan and gets a case of US7-S3 (D46); and D25 takes agent c's
  fix.
- Q: What was not folded? → A: Nothing of D1 to D47 was left out, and four points stay
  as they were, each for a stated reason. The citations' line numbers (D7, D36 and the
  stale anchors the reports list) are the re-anchoring's; D36's two insertion places are
  now also named by their content. Three mutations are equivalent at every instant a
  test can choose and have no case: the same-instant sample of US1's memory, the `mark`
  where a measurement opens (noted in plan.md § The tracker's two answers), and the
  `biased` race in US7's taker. And D26's note that US6-S3's second window [now
  US6-S12's], 6.4 s to 7.0 s, also admits a one-second poll was proposed and not run, so
  that window is unchanged. (The reanalysis ran it: 6.4 s to 6.8 s passes and is the
  window now, EE5.)

### Session 2026-11-19 (the reanalysis)

Six reviews of the folded draft followed round 1's fold on 2026-11-19, each from a side
of its own: the steering as a control loop and a state machine (AA1 to AA14); what it
leans on in 015, 030 to 035 and 038 and what 037 needs of it (BB1 to BB11); the trio's
consistency, coverage and wording (CC1 to CC15); an audit of round 1's defects and their
fixes (DD1 to DD13); a re-run of every folded case with all eight stories built (EE1 to
EE5); and the steering on a real night (FF1 to FF20). They found 78 issues, most of them
the same few problems met from several sides. Only the re-run ran code; the sizes two
reviews give come from their own models of plan.md's control law, one of which gives
round 1's two measured drifts. All 47 of round 1's defects were real and 40 of their
fixes sound, and 59 of its 60 folded cases passed on three runs, the one that failed
lacking only a code change that probe never had. Round 1 had built on a simulator with a
perfect solve, an instant solver, an obedient mount and nobody touching the tube, and
the steering had no rule for four things a real night brings: a hand on the tube, the
switch turned off mid-stack, recovery after cloud, and a mount that does not make
exactly the step it is asked for. The findings came to fifteen choices for the owner;
the rest were folded without one. The fold was made the same day, in two passes. Nothing
of it has been built: plan.md § Prerequisites 5 says what a second probe round must show
before this spec is flipped.

- Q: What did the owner answer? → A: "Take all 15", on 2026-11-19: each of the fifteen
  choices takes the answer recommended to him.
  1. **A hand on the tube.** A push the session reads as moving is a time the tracker is
     not steady (owner decision 17; FR-021; US6-S6).
  2. **A wrong aim point.** The steering does not steer a stack whose first tracking
     error is over 3′ (owner decision 7; FR-014; US4-S8, US7-S7).
  3. **After cloud, the rate.** After a lapse the finder camera's step is still applied
     in the gaps, and the rate stays as the subs left it (owner decision 6; FR-011;
     US3-S11, US9-S11).
  4. **After cloud, the count.** The lapse comes after eight gaps with no tracking
     error, not three (owner decisions 6 and 15; FR-011; US9-S4).
  5. **A mount that is not perfect.** No gap changes an axis's rate by more than 0.1″/s;
     a pair of errors with a large step between them changes none; and a step is booked
     for the time its pulse really lasted (owner decision 4; FR-008, FR-009; US3-S4,
     US3-S8).
  6. **Settling.** The tracker is not steady for 2 s after its first rates, a take-up
     and the keyhole pass (owner decision 17; FR-021; US6-S1, US10-S3).
  7. **A stopped axis.** The tracker reads not steady for a turn that finds an axis at
     rest whose last rate was not zero; the scope node's own fix is another spec (owner
     decision 17; FR-021; US6-S7).
  8. **The exposure limit.** The exposure is decided from the shortest limit of the hour
     ahead, is never halved to under 1 s, and exposures pause while the limit is under
     half the exposure held (owner decision 2; FR-017, FR-018, FR-022; US5-S1, US5-S10,
     US5-S11, US6-S17).
  9. **The default smear** is 6 px (owner decision 1; FR-019; US5-S1, US5-S4).
  10. **The limit before and after a stack.** The view carries the limit, and what a
      stack started now would take, whenever a tracker runs, and neither otherwise
      (FR-018; US5-S4, US5-S12).
  11. **The sentences.** The page's sentences are reworded; a pause says why, in a word
      the tracker gives through `Steered`; and the count of subs dropped as not steady
      is one stack's (FR-021, FR-022, FR-027; US8-S3, US8-S4, US8-S7, US8-S8).
  12. **An agent's reach.** `set_steering` is an MCP tool, and the README says what each
      key of `steering` means (owner decision 11; FR-025, FR-031; User Story 11).
  13. **The gains.** The step share and the rate share are run-only settings (owner
      decision 3; FR-030; US5-S13, US5-S14).
  14. **User Story 3's size.** If the fold's estimate passes 60 KB it is cut by tests
      into a new story, as 035's User Story 5 was. It did (the next answer).
  15. **A second probe round** builds the changed stories before the flip.
- Q: What was cut, and how? → A: User Story 3 stood at about 61,300 B (93% of the 64 KiB
  budget) after the fold's first pass, so User Story 9 takes thirteen scenarios of it,
  in a module of its own, `steering_claim`: three whole and ten that were the later
  halves of scenarios holding more than one Given, When and Then. User Story 6 stood at
  about 55,200 B (84%), and the cases of the second pass, the pause's word, the wait for
  the limit and the windows 037 asks among them, took it to about 60,300 B (92%), so
  User Story 10 takes nine scenarios in the same way, in `steering_unsteady`. Both write
  no code under `crates/finder/src` and implement no FR, and on the code of the story
  they were cut from their scenarios already pass. Estimated, neither cut built: User
  Story 3 about 49,100 B (75%), User Story 9 about 23,900 B (36%), User Story 6 about
  48,400 B (74%) and User Story 10 about 15,300 B (23%) (plan.md § Risks). A scenario
  that moved whole has a new number, and so does each later half; no other scenario's
  number changed but two: US3-S14 took the number 8 that US3-S8 left, so that US3-S9 to
  US3-S11 keep theirs, and US6-S12 took the 11 that US6-S11 left.

  | Was | Now | Module |
  |---|---|---|
  | US3-S2, its second When (subs with feedback beside the noisy task) | US9-S1 | `steering_claim` |
  | US3-S3, its second When and its And (an error over the deadband) | US9-S2 | `steering_claim` |
  | US3-S5, its third When (a step against the axis's motion) | US9-S3 | `steering_claim` |
  | US3-S6, its second, third and fourth bullets (the lapse, a release, errors that keep no claim) | US9-S4, US9-S5, US9-S6 | `steering_claim` |
  | US3-S7, its second and third Givens (a gap before the first rates, a gap nobody waits for) | US9-S7, US9-S8 | `steering_claim` |
  | US3-S8 (the pointing model while steered) | US9-S9 | `steering_claim` |
  | US3-S11, its second When and its second Given (a held step over the limit, a noisy finder camera) | US9-S10, US9-S11 | `steering_claim` |
  | US3-S12, US3-S13 (a followed push) | US9-S12, US9-S13 | `steering_claim` |
  | US3-S14 (the most one gap changes a rate by) | US3-S8 | `steering` |
  | US6-S1, its second Given (the rate in force at a lapsed gap) | US10-S1 | `steering_unsteady` |
  | US6-S2, its second to seventh Givens | US10-S2 to US10-S7 | `steering_unsteady` |
  | US6-S11 (subs steered through a reversal) | US10-S8 | `steering_unsteady` |
  | US6-S12 (a tracker that stops being steady while its gap is awaited) | US6-S11 | `steering_steady` of `console` |
- Q: Which other scenarios were split, and which are new? → A: A scenario may hold one
  Given, When and Then, and a later one becomes a scenario of its own at the end of its
  story, so that the judge grades each case (CC9). Besides the ones the two cuts took:
  US2-S5's second Given is US2-S7; US4-S1's is US4-S9, and US4-S2's second and third
  US4-S10 and US4-S11; US5-S4's is US5-S7 and US5-S6's US5-S8; US6-S3's second to sixth
  are US6-S12 to US6-S16; US7-S1's second When is US7-S8, and US7-S5's two And cases
  US7-S9 and US7-S10; and US8-S6's second Given is US8-S10. A triple begins at a Given,
  or at a When that starts on another rig, steering or console; cases that a Then lists
  for further instances with "the same holds", and steps made one after another on one
  rig, stay in their scenario (US1-S2, US1-S3, US1-S5, US3-S4, US3-S6, US3-S9, US4-S3,
  US5-S1, US5-S2, US5-S5, US6-S4, US8-S1, US8-S4 and US9-S4 among them). New with this
  fold: US1-S7, a push on the mount itself (BB10); US4-S12, a set point changed during a
  stack (AA12, BB7); US5-S9, the switch turned off during a stack (AA5, CC1, DD5, FF2);
  US5-S10 to US5-S12, the hour ahead and what a stack started now would take (choices 8
  and 10); US5-S13 and US5-S14, the two shares (choice 13); US6-S17, the wait for the
  limit (choice 8); US8-S7 to US8-S9, the status's sentences, the pause's and the form's
  drafts (choice 11, DD6); US10-S9, a rate whose time has run out (EE1); and US11-S1 to
  US11-S3, the tool (choice 12). The first pass's were US3-S13 and US3-S14 [now US9-S13
  and US3-S8], US4-S8, US6-S6 to US6-S12 [the last two now US10-S8 and US6-S11] and
  US7-S7. The spec has 113 scenarios in eleven stories, where it had 54 in eight before
  the reanalysis.
- Q: What was folded without a choice? → A: The switch turned off during a stack lets
  the tracker go, and so does a set point that changes (FR-014, FR-019). D40's wiring of
  the stack is rewritten for the 035 that landed (plan.md § The stack). An error
  measured before a follow, or before a time the tracker was not steady, is dropped by
  its time, and a gap served under a lapsed claim always sends each axis its rate
  (FR-010, FR-011). The deadband's 1″ and the step's 30″ are angles on the sky (owner
  decision 5). The guard on `steering.rs` and on 035's `stack/service.rs` reads for
  thirteen names, where it read for six (FR-005; FF20). And the tests and wordings the
  reviews asked for: a gap still queued when tracking ends (US6-S10), the drop with the
  switch off (US6-S4), the readout's window from below (US8-S2), the simulator's memory
  forgetting (US1-S6), the block's fields as drafts (US8-S9), a view right after a
  release (US5-S2), and the window on the rate log (§ User Scenarios).
- Q: Did the fold find anything the reviews had not? → A: Two things. US4-S4's named set
  point, (718.0, 361.0), gave a first tracking error 230″ long, which choice 2's rule no
  longer steers: the scenario now names (668.0, 386.0), 110″ out, and US4-S8 has the
  stack that is not steered. And US5-S1 held a case, a limit of 6 s with a shortest
  exposure of 10 s, that choice 8's wait would have held for good once User Story 6 was
  built, in a test file that story may not edit: the case now has a limit of 8 s, and no
  case of User Story 5 has a limit under half the exposure it answers.
- Q: What does 037 read from this spec now? → A: Five things, each kept for it (plan.md
  § Shapes that existing tests pin). `Steered::steady()` and
  `Steered::steady_between(now − reach, now)`, both through `Session::steered()`, with
  reach the larger of 10 s and the exposure: false from a tracker's start until 2 s
  after its first rates, across a take-up, the keyhole pass, a measurement and a push
  the session reads as moving, and for the 2 s that follow the first three; the instant
  a time not steady ends is not counted (US10-S3, US10-S6, US10-S7, US6-S6).
  `Steering::waiting()`, and `steering.waiting` in the view, true while a
  `before_exposure` waits for the tracker or for its limit, with `steering.pause` saying
  which. `Steering::set_set_point(None)`, which a run calls as it starts its stack
  (US4-S12). And `steering.far_arcsec`, present while the steering stands back from a
  stack that began more than 3′ from its set point. What 037's own trio says of the
  steering bringing in a target stacked off the centre holds only inside those 3′, and
  slowly where an axis would have to back up (FR-008): its sentences are corrected
  there, not here.
- Q: What was not folded? → A: Five points, each for a stated reason. The words
  `get_imaging_camera` gives itself in the tool listing are pinned by a test of 031's
  (FF19's first point): `set_steering`'s own description points at the `steering` part,
  and the README's line for `get_imaging_camera` names it. The caps of 16 remembered
  gaps, 32 remembered exposures and 64 times not steady have no scenario still: two of
  each pass every test, and a case for each costs more bytes in the largest stories than
  it buys (CC15). FF14's floor of 1″ on the deadband has none either: on the simulated
  mount two counts are larger on both axes below 51° of altitude, where every rig is.
  FF12's rule for an axis whose rate is held at zero waits for what probe round 2
  measures (plan.md § Risks). And the statuses of 030 to 035 as drafts, and the line
  numbers of citations, are the re-anchoring's.

## Owner decisions

Each is a design or taste call the plan document leaves open. None is settled. A
different answer is written into the trio and re-validated before the flip, never left
to a story.

1. **The smear's unit and default.** Recommended: pixels of the imaging frame as
   delivered (after binning), at its corner farthest from the set point, 6 px by
   default. It is what 015's readout uses and what the eye judges in a sub; and it needs
   no scale, so the cap can be decided before a stack's first imaging solve, on a
   console that has measured nothing yet. Two things speak against it and are the
   owner's to weigh. The research note records that a limit in arcseconds does not
   change with binning while one in binned pixels doubles
   (`docs/research/2026-11-15-imaging-camera.md` § 7. Field rotation, sub length and the
   drive): at binning 2 the same 6 px is twice the smear on the sky. And the plan
   document's rule 3 says "Thresholds are in arcseconds or relative to a measured
   value". A smear in delivered pixels keeps what rule 3 is for: it holds no focal
   length, pixel size or scale, and what the tracker is asked is a pure angle, the smear
   over the corner's distance in the same pixels. It is not rule 3's letter. Instead:
   arcseconds on the sky, 2″ by default, turned into pixels with 032's measured scale
   or, before one, the scale from the pixel size and the focal length setting, and no
   cap when neither is known; or 1 px, 015's readout figure. Changed by the owner on
   2026-11-19 (the reanalysis, choice 9): the default is 6 px, where it was 2 px. The
   2 px was worked here and sourced nowhere. It widens an in-focus star by 3% to 4%
   along the trail: the research note puts in-focus stars at this telescope at 4.6 to
   5.5 px (`docs/research/2026-11-15-imaging-camera.md` § 6. Focus), a half-flux
   diameter equals a Gaussian's full width, 2.3548 σ, and a uniform trail of L pixels
   adds L²/12 to the variance along it, so the width grows by √(σ² + L²/12)/σ, a factor
   of 1.043 at σ = 1.953 px and 1.030 at σ = 2.336 px. By the same arithmetic 6 px
   widens one by 24% to 34% (1.245 and 1.336), at the far corner only. Three figures of
   the research note's § 7 speak for the larger default, and the decision as drafted
   cited none of them: without a coma corrector that same corner carries 28 px of coma;
   a published worked example allows 7 px in the corners of an APS-C sensor; and the
   owner recalls exposures of about 20 s to 30 s holding with this camera. What a
   default costs is exposure. For the worked example's camera unbinned (a corner
   2,968 px from the centre) at 35° N, the longest exposure in seconds, by minutes from
   the target's transit, is, at 2 px and at 6 px: for M42 (50° of altitude at transit)
   9.5 and 28.6 at 90 minutes before, 8.3 and 24.9 at 60, 7.6 and 22.7 at 30, 7.3 and
   21.9 at transit; for a target at Dec +22° (77°) 9.8 and 29.3, 5.5 and 16.4, 3.2 and
   9.7, 2.5 and 7.6; for M31 (84°) 8.2 and 24.5, 4.8 and 14.5, 2.2 and 6.7, 1.2 and 3.7;
   for M33 (86°) 33.1 and 99.2, 10.3 and 30.8, 2.8 and 8.4, 0.8 and 2.5; and each is the
   same 30 minutes after transit as 30 minutes before. At 2 px a setting of 20 s on M42
   became 5 s; at 6 px it is kept (FF5, FF13). It stays a setting, and the unit question
   is as it was.
2. **What the stack does when the limit is under the exposure asked for.** Recommended:
   before its first sub it halves the exposure as often as the limit asks (8 s, 4 s,
   2 s), stopping at the camera's shortest, and holds that exposure for its whole life:
   035 drops a sub whose exposure differs from the reference sub's, and a night then
   needs a master dark at a few exposures only. The stack sets the console's exposure
   setting to the exposure in use through its own lease on the camera, so the setting
   shows it while the stack runs, and puts the observer's value back through the same
   lease as it ends. If that first change is refused, the stack ends at once with the
   reason `camera`, having taken nothing. If the limit later falls under the exposure
   in use, the readout says so and nothing else changes. Instead: end the stack when
   the limit falls under its exposure, for the observer or 037 to start another; or
   follow the limit sub by sub, which needs 035 to weight subs by exposure and a master
   dark for every length. Changed by the owner on 2026-11-19 (the reanalysis, choice 8),
   in three ways. The exposure is decided from the shortest limit of the hour ahead, not
   from the limit at the instant the stack starts: before its transit a target's limit
   only falls, so a stack asked for 30 s on M31 an hour before transit held 7.5 s as
   drafted and trailed 12 px in the corners at transit, where it now holds 1.875 s and
   trails 3 px (at the default 6 px, for the worked example's camera unbinned at 35° N).
   The exposure is never halved to under 1 s, where the halving stopped only at the
   camera's shortest and a 30 s setting could become subs of a quarter of a second. And
   while the limit is under half the exposure held the stack takes no exposure: it
   waits, lets the tracker go to its own corrections meanwhile, and goes on by itself
   once the limit has risen, so it never stacks subs smeared by more than twice the
   smear chosen. `over_limit` still says when the limit is under the exposure held
   (FR-017, FR-018, FR-022; FF5). Not taken: the pause alone; ending the stack with a
   reason of its own.
3. **Where the settings live** (the plan document's route table, in this worktree, has
   this route as 036's row). Recommended: a route of this spec's own,
   `PUT /api/imaging/steering`, run-only as 031's console settings are; the plan
   document's table already has the row, and another answer corrects it (plan.md
   § Prerequisites 9). Instead: keys of
   `PUT /api/imaging/settings`, which 031's tests pin to nine keys and which the stack's
   lease refuses while a stack runs; the session file; or a flag of the binary. Added by
   the owner on 2026-11-19 (the reanalysis, choice 13): the control law's two shares
   (owner decision 4), the step share, 0.5, and the rate share, 0.3, are settings on the
   same route as well, each a number from 0 to 1, run-only, and the page shows them
   under a `Tuning` heading. As drafted they were constants, and the telescope is two
   hours from where a console is rebuilt: the switch alone gave one comparison, all of
   the feedback or none (FR-030; FF15). The deadband, the step's limit and the rate's
   limit a gap stay constants.
4. **The control law.** Recommended: a step and a rate. In the gap after a tracking
   error, the tracker moves each axis by half of that axis's error when it is outside
   the deadband, as one half-second pulse; and it takes 0.3 of the drift rate measured
   between two subs out of that axis's rate, changing the rate by at most 0.1″/s in one
   gap. Both allow for what the gaps since the sub's mid-exposure already did, so an
   error that arrives late is not corrected twice. A large first offset then cannot wind
   the rate up: a pair of errors between which the gaps stepped an axis by more than two
   deadbands changes no rate on that axis, so a mount that does not make exactly the
   step it was asked for is not taken for one that drifts. Nothing but the feed-forward
   changes during an exposure. Instead: 015's proportional-integral law with other
   gains. Changed by the owner on 2026-11-19 (the reanalysis, choice 5): the 0.1″/s a
   gap, the two-deadband rule, and a step booked for the time its pulse really lasted
   and not for a flat half second, are new. The drafted law believed every step was made
   exactly: in a model of it a 20% error in a 30″ step wound the rate to 0.6″/s, and a
   40″ bump at 30 s subs passed the jump test and cost five subs (AA6, AA7, FF7, FF9).
   The drafted sentence was "A large first offset then never winds the rate up".
5. **The deadband.** Recommended: per axis, two of that axis's counts or 1″ on the sky,
   whichever is larger, so the smallest step asked for is one whole count of the
   controller's reference (1.31″ and 0.65″ on the worked example's mount). The rate has
   no deadband: a rate can be set finer than a count. Instead: one figure in arcseconds
   for every mount. Reworded on 2026-11-19 (the reanalysis, FF14, folded without a
   choice): the 1″, and the 30″ one gap may move an axis, are angles on the sky and not
   of the axis. An axis's own angle is worth less sky the nearer the target is to that
   mount's pole: an azimuth angle is a sky angle over the cosine of the altitude, so 1″
   and 30″ of azimuth were 0.17″ and 5.2″ of sky at 80° of altitude.
6. **When the subs stop steering, and when they steer again.** Recommended: they stop
   when the stack lets go, when eight gaps in a row bring no tracking error, when the
   tracker is not steady, or when tracking ends; counted in gaps, never in seconds. The
   tracking offset is kept on a release and on such a lapse (the end of tracking and a
   keyhole pass zero it, as 015 has them), and the tracker's own corrections, from the
   finder camera on this mount, steer again. After eight gaps with no tracking error a
   stack still holds the tracker, and its own corrections are then applied only in the
   stack's gaps, and only as steps: a finder pointing that arrives between two gaps
   changes no rate, and the next gap applies the step 015's proportional term would have
   made for the newest pointing (its rate over its interval, 0.6 of the error); the
   tracking offset stays as the subs left it. The tracker stays so until an accepted sub
   hands it a tracking error, and the gap after that takes it back and forgets what was
   held. So while subs keep being dropped (cloud, a wrong field) the finder camera
   steers without a break, and the settled rule "rate corrections change only between
   exposures" holds while the subs steer and after they lapse alike. Once the stack lets
   go, or while the tracker is not steady (owner decision 17), 015's corrections run as
   written: no stack is exposing then, or the stack is waiting and drops the sub that
   crossed that time (owner decision 9). Instead: take the tracker back at the very next
   gap, which leaves the finder camera steering during one exposure in nine while subs
   are dropped and nothing steering during the other eight; or, after the eight gaps,
   015's corrections as written, whose rate changes then fall inside the exposures the
   stack goes on taking; or an age in seconds. Changed by the owner on 2026-11-19 (the
   reanalysis, choices 3 and 4). The lapse comes after eight gaps, not three: 035's
   queue of four waiting subs, the one in work and the three of before, because with a
   slow worker tracking errors arrive several gaps apart, and at three the claim lapsed
   between every two of them and the subs never measured a drift rate (AA9, BB6, FF6,
   DD8). And after a lapse the finder camera's step is still held and applied, but the
   change 015's integral would have made to the tracking offset is not: summed over the
   pointings of one gap that integral is unstable past about 25 s between gaps, and at
   any sub length it puts the finder camera's scatter into the rate the subs had learned
   (AA1, FF4). The drafted answers were three gaps, and the integral's changes held and
   added up.
7. **The finder camera while the subs steer.** Recommended: its live pointings change no
   rate, whatever they say; they still feed the pointing model, the cable watch, the
   keyhole's resume, the backlash measure and 015's readout. Once the subs have lapsed
   they change a rate only through a gap (owner decision 6). Instead: let a finder error
   over some bound take the tracker back. Added by the owner on 2026-11-19 (the
   reanalysis, choice 2): the finder camera holds the target on its aim point and the
   subs hold it on the set point, and where the two differ by more than 038's 5′ the
   subs' own centring would read as a push by hand, again and again. So the steering
   does not steer a stack whose first tracking error is over 3′ (180″): it lets the
   tracker go, the tracker's own corrections run as 015 has them for the rest of that
   stack, and the page says to centre the target or apply the measured aim point
   (FR-014; BB5). Not taken: holding the field wherever the stack began.
8. **A switch.** Recommended: `on`, true by default, in the same route. Turned off, the
   stack hands the tracker no error and claims nothing, and the exposure cap and the
   pause still apply. The field check "with and without the feedback" needs it. It is
   the observer's declared exception to the settled rule that rate corrections change
   only between exposures: with the switch off no stack holds the tracker, so the
   tracker's own corrections run as 015 has them, during exposures as well, which is
   what the field check compares the feedback with. Instead: no switch; or a switch that
   still holds the tracker, its own corrections applied in the gaps as after a lapse.
   Made true on 2026-11-19 (the reanalysis, folded without a choice): as drafted the
   switch only stored a flag, so turned off while a stack ran it left the tracker
   claimed with nothing steering it, neither the subs nor the finder camera, which is
   the very state the field check would have read. Turning it off now lets the tracker
   go at once, and turned on again the stack's next gap claims it (FR-019; AA5, CC1,
   DD5, FF2).
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
    story after US5. Changed by the owner on 2026-11-19 (the reanalysis, choice 12):
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
13. **The simulator's count steps.** Recommended: off by default, so no existing
    simulator figure moves. Instead: on by default.
14. **The part's name, its file and its words** (the plan document's list of the parts
    of the session's `imaging` object and its module tree, in this worktree, name
    `steering` and `steering.rs` for 036). Recommended: `steering`, the plan document's
    own verb, for the part of the imaging view and for the module `crates/finder/src/imaging/steering.rs`, with the
    sources `subs`, `tracker` and `none`: the imaging path says "the tracker's own
    corrections" and never "the finder camera", which another telescope may not have.
    The plan document's two lists already carry the name, and another answer corrects
    them (plan.md § Prerequisites 9). Instead: `tracking`, which would give the console
    two files of that name; or the source `finder`.
15. **Which subs steer.** Recommended: only a sub the stack accepted, as the plan
    document has it: a wrong solve that 035 refuses never reaches the mount, and eight
    gaps in a row with no accepted sub hand the tracker back to its own corrections
    (three as drafted: changed by the owner on 2026-11-19 with owner decision 6).
    Instead: also a sub dropped as `soft` or `off_field`, whose solve is as good as any;
    that would let the subs pull back a drift that is itself what made them soft.
16. **How long the simulator remembers.** Recommended: one hour of its clock, the
    simulated camera's longest exposure. Instead: a count of samples.
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

## Owner approvals

Flipping this spec to `ready` approves the recommended answer to every owner decision
above, and the following. Each is named in the FR or success criterion that makes it.

1. **015's out-of-scope line is superseded.** `specs/015-tracking/spec.md:763`, "guiding
   from the imaging camera; flexure between the finder camera and the tube limits
   imaging-grade guiding anyway", no longer holds: this spec is that work. No file of
   015's trio is edited (FR-007).
2. **015's FR-008 is suspended while a stack holds the tracker.** "The console SHALL
   correct the axis rates from at least one solve every 5 s"
   (`specs/015-tracking/spec.md:618`) then means: while the subs steer, the subs'
   solves, at the stack's cadence, the finder camera's solves correcting nothing; and
   once the subs have lapsed, the finder camera's solves, applied only in the stack's
   gaps (owner decision 6). In every state in which the stack's gaps are served, no
   solve changes a rate between two gaps. Once the stack has let go, and while the
   tracker is not steady, FR-008 holds as written (FR-007, FR-010, FR-011, FR-021). No
   existing test changes.
3. **`PUT /api/imaging/steering`**, the `steering` part of the imaging view and of the
   session's `imaging` object, which the MCP tools that return either carry too, the
   module `crates/finder/src/imaging/steering.rs`, and, since 2026-11-19, the MCP tool
   `set_steering` (FR-005, FR-016, FR-019, FR-031). The plan document's § Interfaces, in
   this worktree, names the first three and no MCP tool for 036; owner decision 11's
   answer of 2026-11-19 corrects its list of tools in the same change as the flip
   (plan.md § Prerequisites 9).
4. **Two `GLOSSARY.md` terms**, Tracking error and Set point (FR-006).
5. **The simulator's imaging truth changes for a moving mount** (FR-001): 030's
   `imaging_truth_at` then answers with the direction the telescope had at the instant,
   and as before on a still mount. And the simulated mount gains count steps, off by
   default (FR-002).
6. **035's stack changes in four ways** (FR-023): `Dropped` gains a twelfth count,
   `not_steady`, written last and always; a sub may end with the outcome `not_steady`;
   the taker waits for the steering before every exposure; and before a stack's first
   sub the taker may set the console's `exposure_ms` to a shorter exposure than the
   observer set, through the stack's own lease, the taker putting the observer's back
   through the same lease whenever it returns, before the lease drops with it. A stack
   whose first change of the exposure is refused ends with the reason `camera`. No
   existing test changes: 035's tests
   read `dropped` key by key and never assert how many keys it holds (035 spec.md
   § User Scenarios), and plan.md § Prerequisites 3 checks that in 035's landed tests
   before the flip.
7. **The Imaging page's block `Tracking from the subs`**, its words, its `DESIGN.md`
   entry and its component in `.impeccable/design.json`; and the words
   `while the mount was not steady` in 035's `Dropped` line (FR-027, FR-028, FR-029).
8. **A README section, a route row and the lines of one MCP tool** in
   `crates/finder/README.md` (FR-020, FR-025, FR-031).
9. **Each new test module is declared in an existing test file** (SC-002): one `mod`
   line a module, in alphabetical order, in `crates/finder/tests/console/main.rs`
   (`steering_cap`, `steering_error`, `steering_gap`, `steering_readout`,
   `steering_steady`), `crates/finder/tests/motion/main.rs` (`steering`, `steering_cap`,
   `steering_claim`, `steering_gap`, `steering_stack`, `steering_steady`,
   `steering_unsteady`) and `crates/finder/tests/mcp/main.rs` (`steering`), as 027's
   layout asks of every finder test. `steering_claim` and `steering_unsteady` are the
   modules of User Stories 9 and 10, cut from User Stories 3 and 6 on 2026-11-19, and
   `steering` of `mcp` is User Story 11's: the approval's own rule, one `mod` line for
   each new finder test module, applied to three more. No other line of those files and
   no other existing test changes.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions unless it says otherwise.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's scenarios run in
  `npm test`, a later command of the same gate, in `crates/finder/web`.
- **Paused time.** Every Rust test here runs under
  `#[tokio::test(start_paused = true)]`, so every wait is in virtual time.
- **A position in an imaging frame**, angles "east of north" and **a separation** are as
  032 spec.md § User Scenarios defines them: (x, y) in pixels from the frame's top-left
  corner, the centre of a frame W wide and H high at (W/2, H/2), up toward smaller y.
- **A sky offset** of a place P from a place Q is east = (RA of P less RA of Q, reduced
  into −180° to 180°) × cos(Dec of Q) × 3600 and north = (Dec of P less Dec of Q) ×
  3600, in arcseconds, as `crates/finder/tests/support/tracker.rs:89` computes it.
- **The worked solve** is `ImagingSolve::from_cd(83.8221, -5.3911,
  [[-5.196152422706632e-4, -3.0e-4], [3.0e-4, -5.196152422706632e-4]], 1236, 822)`: a
  frame 1236 × 822 centred on RA 83.8221°, Dec −5.3911°, up 30° east of north, 2.16″ a
  pixel, not mirrored (032's worked answer B). **The worked target** is M42's catalogue
  place, RA 83.81867°, Dec −5.38967° (`crates/finder/data/openngc.csv:7666`). **The
  worked frame** is a `SolveFrame` 1236 × 822, 16 bits, `RGGB`, binning 1, whose `roi`
  is the whole sensor (0, 0, 1236, 822), with the `start` its scenario gives, an
  `exposure_s` of 5.0 unless it gives another, and any path.
- **The tracker rig** is the rig `crates/finder/tests/motion/tracking/pi_loop.rs:104`
  builds, written out again in the module that uses it, with two changes: the base tilt
  is 0° in the mount and in the model, and it keeps its `Tracker` in an `Arc`, so that a
  second task or a `Steering`'s source can share it. A module keeps the rig's
  `Simulator` as a field only where it reads it (US4-S5's does; US3's does not, and a
  field nothing reads fails the `lint` gate). So: a
  `VirtualClock` from Unix 1768528800 (`2026-01-16T02:00:00Z`); the target M42; a
  `SimMount` with no backlash whose zero points put its axes on M42's apparent place at
  the start, and whose truth is loaded into the model with `load_known_terms`
  (`crates/finder/src/model.rs:347`); a wrapper round the mount that logs every rate
  command with the console's time and its axis, as
  `crates/finder/tests/motion/tracking/pi_loop.rs:41` does: **the rate log**; frames
  stopped; and a `Tracker` on the console's clock at the site 40° N 75° W with a
  minimum altitude of 15°, started on M42. A scenario gives the mount's rate error.
  **The rig waits for the first rates:** after `start` answers, it looks at the rate log
  every 10 ms of tokio time until the log holds at least one rate for each axis, and
  fails if that takes more than 5 s; then it waits 3.5 s more. A tracker's first rates
  are sent by its task's first turn, not by `start` (Clarifications), and a gap asked
  for before them moves and claims nothing. The 3.5 s are for US6, under which a tracker
  settles for 2 s after its first rates and turns a gap away until then (FR-021): US3's
  scenarios must pass before US6 and after it. Every scenario on the tracker rig begins
  after that wait, unless it says the rig is built without it. Each module that writes
  the rig out writes the wait too.
- **Windows on the rate log.** Its times are console Unix seconds near 1.77 × 10⁹, where
  an `f64` resolves about 2.4 × 10⁻⁷ s: a test that tells two instants apart, or opens a
  window just after one, adds at least 1 ms, never 10⁻⁹ s, which is lost in the sum.
  Where a scenario says "every rate" in a window, its test also asserts that the window
  holds at least one rate for each axis it speaks of.
- **The truth error** on the tracker rig is the sky offset of the mount's true
  direction, as a J2000 place, from M42's apparent place, as
  `crates/finder/tests/motion/tracking/pi_loop.rs:217` computes it. **The feed-forward
  rate** of an axis at a time is the test's own, as
  `crates/finder/tests/motion/tracking/pi_loop.rs:252` computes it.
- **Playing a sub** of exposure T on the tracker rig is: (1) await `tracker.gap()` and
  keep what it answers; (2) read the truth error e0; (3) wait T/2 in 50 ms steps and
  read the truth error em and the console time tm; (4) wait T/2 and read e1; (5) wait
  1 s; (6) call `tracker.steer` with the time tm and em's east and north. The sub's
  **true drift** is the length of e1 − e0. **Playing a sub without feedback** is steps 2
  to 5 alone.
- **An axis displacement** of (a, b) arcseconds at time t is the sky offset, computed by
  the test with the simulator's transforms (`crates/finder/tests/support/tracker.rs:76`,
  `crates/finder/tests/support/tracker.rs:83`), of the direction at altitude
  `alt + b/3600` and azimuth `az + a/3600` from the direction at (alt, az), M42's
  apparent place at t. **One count** of an axis is 1,296,000 divided by the counts a
  turn the mount reports for it: 0.8″ on the simulated mount
  (`crates/simulator/src/mount.rs:12`).
- **Playing a sub late** is playing a sub with step 6 changed: for the first sub the
  test hands over nothing; for every later sub it hands over the em and tm it kept
  from the sub before it. So each tracking error reaches the tracker one sub after its
  own, as 035's worker delivers them (Clarifications).
- **The backlash rig** is `crate::support::tracker::Rig`
  (`crates/finder/tests/support/tracker.rs:98`) for M42 from 120 s before its transit
  (`crates/finder/tests/support/tracker.rs:237`), on a session file that already holds
  0.3° of backlash for both axes, as
  `crates/finder/tests/motion/tracking/backlash.rs:71` writes one. `Rig::follow`
  (`crates/finder/tests/support/tracker.rs:269`) feeds it the truth as finder pointings.
  **The lapsed backlash rig** is the backlash rig followed to 100 s before the transit
  with `rig.follow(transit - 100.0, 60.0)`, with one `gap` awaited then and eight more
  6 s apart with no `steer`: its claim lapsed at the ninth gap, 52 s before the transit,
  and `steered_by_subs()` is false.
- **The scripted tracker** is a type of the test module's own that implements `Steered`:
  it answers a target the test sets; logs every `steer`, `gap`, `release` and, where its
  module's tests read them, `tune`, with the console's time; makes `gap` wait a time the
  test sets, on tokio time, and answer a `Stepped` the test sets; says it is steered by
  the subs from its first `gap` to its next `release`; answers the longest exposure,
  `steady`, `steady_between` and the `pause` word the test sets; and keeps the last
  window `steady_between` was asked about. Unless a scenario says otherwise its target
  is the worked target, its `gap` waits 0 s and answers (0.0, 0.0), it names no longest
  exposure, `steady` and `steady_between` are true and `pause` is none. It answers
  `longest_exposure_ahead_s` what it answers `longest_exposure_s`, unless a scenario
  sets a longest exposure for the hour ahead apart. **A
  scripted steering** is a `Steering` whose source answers that tracker from a cell the
  test can empty, so that the test's own `Arc` is the last one; it runs on the console's
  `FixedClock` (`crates/finder/src/session.rs:41`) at 1768528800.0, or on the rig's
  clock where a scenario moves a mount, or on a clock that follows tokio's where a
  scenario says so. **The worked ask** is `ExposureAsk { wanted_s: 5.0,
  shortest_s: 0.001, sensor: (1236, 822), roi: the whole sensor (0, 0, 1236, 822),
  binning: 1 }`, with the `wanted_s` its scenario gives where that is not 5 s.
- **A star's centroid** in a simulated imaging frame taken with noise off is the mean
  position of the samples within 10 px of the brightest sample, in x and in y, each
  weighted by its value less the background the scenario states, a sample's position
  being its centre, (column + 0.5, row + 0.5). 030 US2-S3 holds the same quantity
  within 0.3 px of the truth.
- **The tracking console** is the console `crate::support::goto::Rig` builds
  (`crates/finder/tests/support/goto.rs:113`) at altitude 50°, azimuth 120°, pointed at
  M42 with `point_eyepiece_at`, synced, and tracking M42 through `POST /api/track` with
  no GoTo, as `crates/finder/tests/motion/tracking/routes.rs:96` does. A scenario that
  says "before Track is turned on" takes it before that last step.
- **Scratch folders** come from `crate::support::Scratch::new(name)`
  (`crates/finder/tests/support/mod.rs:14`).
- **Requests** go through the router in process, as `request` in
  `crates/finder/tests/support/live.rs:90` sends them, with
  `Content-Type: application/json` and no `Origin`. A foreign `Origin` is
  `http://elsewhere.example`, sent with a `Host` of localhost on port 8080.
- **Comparing a view.** A test reads `steering` key by key, never whole: later stories
  add keys.
- **Oracles.** Each scenario names its oracle: the simulator's truth, a worked example
  whose arithmetic is given, 015's own functions where a scenario says this spec must
  agree with them, or the words, statuses and keys the scenario itself states.

### User Story 1 - The simulator remembers where the telescope pointed (Priority: P1)

As a test of tracking from subs, I ask the simulator where the imaging camera pointed
at the middle of an exposure that ended seconds ago, while the mount has gone on moving,
and I get the truth of that moment. I can also make the simulated mount move in whole
counts.

**Why this priority**: Without it every simulated imaging solve on a tracking mount is
wrong by more than the error this spec corrects, and no later scenario means anything.

**Independent Test**: `crates/simulator/tests/imaging_history.rs`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p simulator --locked --test imaging_history`.

**Acceptance Scenarios**:

1. **Given** a `Simulator` over the test database at 40° N 75° W on a `VirtualClock`
   from 1768528800, a `SimMount::with_config` with no backlash and no base tilt
   connected, `push_to(50.0, 120.0)`, `set_rate_error(0.25)` called on the mount and
   only then its azimuth axis commanded at 0.02°/s (a rate error applies to the rate
   commands that follow it, `crates/simulator/src/mount.rs:302`), 5 s waited, and a
   `SimImagingCamera` with noise off, gain 0 and one white star of magnitude 8 at the
   centre of `sim.imaging_truth()` as read then, **When** a test starts a 2 s exposure
   in a task with no wait in between, reads `sim.imaging_truth()` and
   `sim.telescope().eyepiece` (its RA, Dec and azimuth) at its start, 1 s later and 2 s
   later, waits 30 s more with the axis still turning, and asks `sim.imaging_truth_at`
   for those three instants, **Then** a committed test asserts that:
   - each answer's centre is within 0.02″ of `sim.telescope().eyepiece`'s RA and Dec
     read at that instant, and its angle within 0.001° of the `imaging_truth()` reading
     taken then (oracle: the simulator's own truth, read live);
   - the eyepiece's azimuth rose by 0.0500° ± 0.0005° over the 2 s (worked example:
     0.02°/s × 1.25 × 2 s);
   - the first and third answers' centres are more than 60″ apart, so the solve's centre
     moves with the drift;
   - the star's trail in the frame is as long as the truth says. The star sits on the
     first truth's centre, so it starts at the frame's centre, (618.0, 411.0). With d
     the separation of the first and third answers' centres in arcseconds over the
     truth's scale, A the position angle from the third answer's centre to the first's
     (`atan2(sin Δα·cos δ1, cos δ3·sin δ1 − sin δ3·cos δ1·cos Δα)`, Δα the first's RA
     less the third's, east of north) and θ the third answer's angle, it ends at
     P = (618.0 − d·sin(A − θ), 411.0 − d·cos(A − θ)). The test asserts that d is above
     20; and, with background 170 (160 counts of offset and 2 s of sky at 5 counts a
     second, by 030's constants) and over the samples whose centres lie within
     d/2 + 10 px, in x and in y, of the midpoint of (618.0, 411.0) and P, that the
     larger eigenvalue of the second moments of those samples, each weighted by its
     value less the background, less the smaller is within 10% of d²/12, and that
     their weighted mean position is within 1.0 px of that midpoint (oracle: the
     simulator's truth; the measure is 030 US2-S6's second half, with a window sized
     to the trail, since 41 × 41 samples do not hold one of 40 px or more; the test
     computes the separation and the angle itself, from the truths' RA and Dec).
2. **Given** two simulators built as in US1-S1, the same instant apart from nothing,
   each with its altitude axis also commanded at 0.01°/s after its azimuth axis,
   **When** a test reads the first's `imaging_truth()` at t and t + 2 s only, and the
   second's `imaging_truth()` and `telescope().eyepiece` at t, t + 1 s and t + 2 s,
   **Then** a committed test asserts that the first's `imaging_truth_at(t + 1.0)`, asked
   at t + 2 s, has its centre within 0.02″ of the second's `telescope().eyepiece` RA and
   Dec read at t + 1 s, and its angle within 0.001° of the second's `imaging_truth()`
   read then (oracle: a twin read live, its centre through 030's `directions_at`, which
   FR-001 leaves alone, since `imaging_truth()` itself is answered from the memory; a
   constant rate makes a straight line between samples exact to well under 0.02″).
   **And** for twins pushed to (50.0, 359.85) in place of (50.0, 120.0), whose camera
   azimuth crosses 0° between t and t + 2 s, the same holds.
3. **Given** the simulator of US1-S1 after 10 s of turning, **When** a test reads the
   truth at t, waits 1 s, calls `push_to(40.0, 200.0)`, and asks `imaging_truth_at(t)`,
   **Then** a committed test asserts the answer's centre equals
   `sim.directions_at(t).eyepiece`'s RA and Dec within 10⁻⁹ degrees: a push by hand
   starts the memory again, and the answer is the one 030 gives (oracle:
   `crates/simulator/src/optics.rs:120`); and that 2 s after the push, after one read of
   `imaging_truth()` 1 s after it, `imaging_truth_at(t)` still equals
   `sim.directions_at(t).eyepiece`'s RA and Dec within 10⁻⁹ degrees. The same holds,
   each on a fresh simulator of US1-S1, when in place of the push the test calls
   `sim.set_optics` with `Optics { eyepiece_px: (600.0, 500.0), ..Optics::default() }`,
   `sim.connect_mount` with a second mount built as US1-S1's, or
   `sim.point_eyepiece_at(83.81867, -5.38967)`.
4. **Given** a simulator with no mount, pointed with `push_to(50.0, 120.0)`, **When** a
   test waits 20 s, reading `imaging_truth()` every second, and asks
   `imaging_truth_at(t)` for t 7.5 s back, **Then** a committed test asserts the answer
   equals `sim.directions_at(t).eyepiece`'s RA and Dec within 10⁻⁹ degrees (oracle: 030
   US2-S2's own).
5. **Given** a `SimMount::with_config` with no backlash and no base tilt, **Then** a
   committed test asserts that `count_steps()` is false; that with the azimuth axis at 0.0001°/s for
   20 s, `true_direction().az` read every 100 ms takes more than 100 distinct values;
   and that after `set_count_steps(true)` on a mount built the same way, the same 200
   readings take between 8 and 11 distinct values, each a whole multiple of
   360/1,620,000° from the first within 10⁻⁹ degrees (worked example: 0.36″/s for 20 s
   is 7.2″, nine counts of 0.8″); and that, with count steps on, the reading 2.0 s after
   the rate command still equals the first: 0.72″ of drive is under one count of 0.8″,
   and the tube follows its drive rounded down; and that on a third mount built the same
   way, with count steps on and the altitude axis at 0.0001°/s, `true_direction().alt`
   read every 100 ms for 20 s takes between 8 and 11 distinct values.
6. **Given** the simulator of US1-S1, **When** a test reads `imaging_truth()` at t and
   then every 100 s for 3000 s with the axis still turning, **Then** a committed test
   asserts that `imaging_truth_at(t)` is within 0.02″ of the reading at t; and that
   after further reads every 100 s up to t + 3700 s, `imaging_truth_at(t)` equals
   `sim.directions_at(t).eyepiece`'s RA and Dec within 10⁻⁹ degrees: a sample older than
   the hour is dropped, and the answer is then the one 030 gives (oracle: the
   simulator's own truth, read live, and `crates/simulator/src/optics.rs:120`; owner
   decision 16's hour).
7. **Given** the simulator of US1-S1 after 10 s of turning, **When** a test reads
   `imaging_truth()` at t, waits 1 s, pushes the tube on the mount itself with
   `push_by(0.0, -1.0)` (`SimMount::push_by`, a degree down), reads `imaging_truth()`
   once more and asks `imaging_truth_at(t)`, **Then** a committed test asserts that the
   answer's centre is within 0.02″ of the reading at t and more than 1000″ from the
   reading after the push: a push made on the mount is a motion like any other and keeps
   the memory, where a push made through the simulator starts it again, as US1-S3 has it
   (oracle: the simulator's own truth, read live; FR-001).

---

### User Story 2 - The tracking error from a sub (Priority: P1)

As the stack, I turn a sub's imaging solve into a tracking error: how far the sky at
the set point is from the target, east and north in arcseconds, at the sub's
mid-exposure. I hand it to whatever tracker is running through one interface that says
nothing of axes.

**Why this priority**: It is the signal everything else acts on, and the interface is
what keeps the imaging path fit for another mount.

**Independent Test**: `crates/finder/tests/console/steering_error.rs`, the module
`steering_error` of finder's `console` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console steering_error::`.

**Acceptance Scenarios**:

1. **Given** the worked solve and the worked target, **When** a test calls
   `tracking_error(&solve, (618.0, 411.0), (83.81867, -5.38967), 1768528802.5)`,
   **Then** a committed test asserts `time` 1768528802.5 exactly, `east_arcsec`
   12.2934 ± 0.001 and `north_arcsec` −5.1480 ± 0.001 (worked example: the frame's
   centre is 0.00343° of right ascension east of M42 at Dec −5.39°, and 0.00143° south).
2. **Given** the same, with the set point (718.0, 361.0), **Then** a committed test
   asserts `east_arcsec` −120.768 ± 0.001 and `north_arcsec` 196.383 ± 0.001 (worked
   example: 100 px right and 50 px up of the centre is 3600 × (−0.0369615°, 0.0559808°)
   by the solve's matrix, added to US2-S1's answer).
3. **Given** a sensor 1236 × 822, **Then** a committed test asserts that
   `set_point_in_frame` answers (618.0, 411.0) for no set point, the whole sensor and
   binning 1; (109.0, 105.5) for no set point, the region (400, 200, 400, 400) and
   binning 2; and (718.0, 361.0) for the set point (718.0, 361.0), the whole sensor and
   binning 1; and (618.5, 411.5) for no set point on a sensor 1237 × 823, its whole
   region and binning 1 (the centre is half the size, not half rounded down).
4. **Given** a `Simulator` as `crates/finder/tests/support/live.rs:41` builds it, its
   truth at t, an `ImagingSolve::from_cd` the test builds from that truth with the
   matrix `[[−s·cos θ, −s·sin θ], [s·sin θ, −s·cos θ]]` (s the truth's scale in degrees,
   θ its angle), and a place T 100″ from the truth's centre at position angle 70°,
   **When** a `SimImagingCamera` with noise off, gain 0 and one white star of magnitude
   5 at T takes a 20 ms frame, **Then** a committed test asserts that with the star's
   centroid, taken with background 160, as the set point and T as the target the
   tracking error is within 0.7″ of (0, 0) (oracle: the rendered frame: the target's
   own image is where the error is nothing; 0.3 px at 2.18″ a pixel); and that with
   the frame's centre, (618.0, 411.0), as the set point it is (−93.97, −34.20) ± 0.05″
   (worked example: −100″ × (sin 70°, cos 70°)).
5. **Given** the worked solve, **Then** a committed test asserts that the target at the
   far side of the sky from the solve's centre, (263.8221, 5.3911), answers `None` (032's
   `pixel_of` answers none only where the cosine of the distance from the centre is
   zero or less; a target exactly a quarter turn away sits on that edge and is not
   used here), and so does the worked target with a time that is not finite, and so does
   the worked target with the set point (NaN, 411.0), and the target (NaN, −5.38967)
   with the set point (618.0, 411.0); and so does the worked target on
   `ImagingSolve::from_cd(83.8221, -5.3911, [[0.0, 0.0], [0.0, 0.0]], 1236, 822)`, whose
   results are not finite.
6. **Given** the text of `crates/finder/src/imaging/steering.rs`, read by the test from
   `CARGO_MANIFEST_DIR`, **Then** a committed test asserts it holds none of `ephemeris`,
   `alt_az`, `parallactic`, `counts_per_revolution`, `AxisInfo`, `MountClient`,
   `azimuth`, `altitude`, `field_rotation`, `hour_angle`, `meridian`, `backlash` and
   `keyhole` (oracle: the plan document's rule 1; the last seven names were added on
   2026-11-19, since a file could have said any of them and passed).
7. **Given** a type of the test's own that implements the five required methods of
   `Steered` and no other, **Then** a committed test asserts that
   `longest_exposure_s(0.001)` and `longest_exposure_ahead_s(0.001)` are `None`, that
   `steady()` is true, `steady_between(0.0, 1.0)` is true and `pause()` is `None`, and
   that `tune(0.5, 0.3)` returns: a tracker that says nothing limits nothing, is always
   steady and takes no tuning (oracle: FR-005).

---

### User Story 3 - The tracker steered by tracking errors (Priority: P1)

As the tracker, once a stack claims me I take my corrections from the tracking errors it
hands me: in each gap I move each axis by half of its error when that is two counts or
more, and I take 0.3 of the drift rate I measure between two subs out of its rate. An
error that reaches me late is judged against what my gaps did since it was measured. I
change nothing else until the next gap, and while a stack holds me the finder camera's
pointings change no rate between my gaps. Once the stack has gone quiet for eight gaps I
take my own corrections again, but only in its gaps and only as steps: the step a finder
pointing asks for waits for the next gap, and my rate stays as the subs left it. I stay
so until the stack hands me a tracking error again, or lets me go.

With round 1's cases and the reanalysis's folded in, this story's estimate passed
60,000 B of the 64 KiB budget, so by the owner's choice 14 of 2026-11-19 thirteen
scenarios are User Story 9's: US3-S8, US3-S12 and US3-S13 whole, and the later halves of
US3-S2, US3-S3, US3-S5, US3-S6, US3-S7 and US3-S11, which each held more than one Given,
When and Then. What they test is still this story's code. The scenario that was US3-S14,
the most one gap changes a rate by, took the freed number 8, so that US3-S9, US3-S10 and
US3-S11 keep theirs (§ Clarifications, the session of the reanalysis).

**Why this priority**: This is the change that stops the smear.

**Independent Test**: `crates/finder/tests/motion/steering.rs`, the module `steering` of
finder's `motion` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test motion steering::`.

**Acceptance Scenarios**:

1. **Given** the tracker rig with a rate error of 0.01, **When** a test plays 40 subs of
   5 s, plays 40 subs of 5 s late on a second rig built the same way, and plays 20 subs
   of 5 s without feedback on a third, **Then** a committed test asserts that:
   - on the first rig, the first sub's true drift is at least 0.6″ (worked example: 1%
     of M42's 14.97″/s across the sky is 0.1497″/s, 0.749″ in 5 s);
   - on the first and on the second rig, every sub's true drift from the 12th on is at
     most 0.15″, and the length of every truth error em from the 12th sub on is at most
     3.0″ (worked example: each drift rate measured cuts the rate error to 0.7 of itself,
     and the 12th sub played late is the 9th such cut, 0.749″ × 0.7⁹ = 0.030″);
   - on the first rig, after the 40th sub, `state().tracking_offset[0]` is within 10%
     of −0.0099 times the azimuth feed-forward rate then, in arcseconds a second
     (worked example: an axis that runs 1.01 times its command needs its command cut by
     1/1.01), and `steered_by_subs()` is true after the first gap;
   - on the third rig, every true drift is at least 0.6″ and the tracking offset stays
     `[0.0, 0.0]` (oracle: the simulator's truth).
2. **Given** the tracker rig with a rate error of 0.01 and a second task that calls
   `tracker.pointing` every 2 s with the truth error plus Gaussian noise of σ 15″ on
   each of east and north, from a seeded generator of the test's own as
   `crates/finder/tests/motion/tracking/pi_loop.rs:289` has, **When** a test plays 20
   subs of 5 s without feedback, **Then** a committed test asserts that the median true
   drift is above 3.0″ (oracle: the simulator's truth; the noise is the research note's
   0.3′ to 0.5′ a finder solve. US9-S1 plays subs with feedback beside the same task).
3. **Given** the tracker rig with no rate error, **When** a test awaits one `gap`, hands
   the axis displacement (0.0, 1.9 counts) as a tracking error at the time now, and
   awaits a second `gap`, **Then** a committed test asserts that the second gap answers
   a `Stepped` of exactly (0.0, 0.0), that the tracking offset is exactly `[0.0, 0.0]`
   (a first error sets no rate), and that the truth error just after it is within 0.05″
   of the one just before (oracle: the simulator's truth and its transforms; the
   deadband is the larger of two counts, 1.6″ here, and 1″ on the sky, which at M42's
   43° of altitude is 1.37″ of azimuth and 1″ of altitude: two counts on both axes.
   US9-S2 has an error over it, on each axis).
4. **Given** the tracker rig with no rate error, **When** a test awaits a `gap`, hands
   the tracking error (12.0, −16.0) and awaits a second, **Then** a committed test
   asserts a `Stepped` within 0.1″ of (−6.0, 8.0) and a tracking offset of `[0.0, 0.0]`:
   a first error sets no rate. **When** it then waits 6 s, hands (6.0 + 2.0, −8.0) at
   the time now, and awaits a third gap, **Then** the `Stepped` is within 0.1″ of (−4.0,
   4.0) and the tracking offset is exactly `[0.0, 0.0]`: the second error shows 2″ of
   drift to the east since the first, but the step made between the two, about 5.2″ of
   azimuth and 9.2″ of altitude, is over two deadbands (3.2″ here) on both axes, and
   such a pair sets no rate. A mount that made a little more or less than the step asked
   for would show the same. **When** it waits 6 s more, hands (4.0, −4.0) at the time
   now, where that step left the target, and awaits a fourth, **Then** the `Stepped` is
   within 0.1″ of (−2.0, 2.0) and the tracking offset is still exactly `[0.0, 0.0]`: the
   step between, about 3.9″ and 4.9″, is over two deadbands on both axes again. **When**
   it waits 6 s more, hands (2.0 + 1.3, −2.0) at the time now and awaits a fifth,
   **Then** each axis's tracking offset is within 0.002″/s of −0.3 × d/Δt, where Δt is
   the time between the fourth and third errors' times as the test handed them, and d is
   that axis's part of a sky offset of (1.3, 0.0), by the differencing of
   `crates/finder/tests/motion/tracking/pi_loop.rs:481` (oracle: the simulator's horizon
   transform, differentiated by the test. The step between these two errors, about 2.0″
   of azimuth and 2.4″ of altitude, is under two deadbands on both axes, and the two
   changes, about +0.079″/s and +0.015″/s, are under the 0.1″/s one gap may make).
5. **Given** the tracker rig with no rate error, a first gap, the error (0.0, 0.0)
   handed over at the time now, a second gap, 6 s waited and then the axis displacement
   (−20.0″, 0.0) handed as an error at the time now, **When** a third gap is awaited,
   **Then** a committed test asserts that the tracking offset is within 0.002″/s of
   `[0.0, 0.0]` (a jump of about 3″/s on the azimuth axis is over twice the 0.752″/s the
   offset may hold, so it is not taken for a drift rate; the tolerance is for the
   altitude axis, where the tracker's own resolving of the same displacement leaves a
   few hundredths of an arcsecond, which is a drift rate of thousandths) and that the
   `Stepped` is within 0.2″ of −0.5 times the error. **When** after 6 s more the axis
   displacement (−100.0″, 0.0) is handed over at the time now and a fourth gap awaited,
   **Then** the `Stepped`'s length is within 3% of the length of the axis displacement
   (30.0″/cos a, 0.0), a being the altitude of M42's apparent place then: one gap moves
   an axis 30″ on the sky at most, which is 41.0″ of azimuth here; were the limit an
   angle of the axis, the step would be 22.0″ on the sky (oracle: the simulator's truth
   and transforms; the displacements are negative so that the step runs with the azimuth
   axis, which turns forward at M42's place then. US9-S3 has a step against the axis's
   motion).
6. **Given** the tracker rig with no rate error and a first gap, **Then** a committed
   test asserts that `tracker.pointing(30.0, 0.0)` called every 2 s for 20 s changes no
   rate: every rate in the log over those 20 s equals its axis's feed-forward rate
   within 0.001″/s, and the tracking offset stays `[0.0, 0.0]`; that `state().samples`
   holds at least 9 samples of 30″; and that on another rig,
   `tracker.pointing(30.0, 0.0)` called just before the first gap leaves every rate the
   log gains in the 2 s after that gap's answer equal to its axis's feed-forward rate
   within 0.001″/s: the claim ends the finder camera's proportional rate (oracle: the
   rig's rate log and the test's own feed-forward rate. The lapse, a release and the
   errors that keep no claim are US9-S4, US9-S5 and US9-S6).
7. **Given** the tracker rig with a rate error of 0.01 and ten subs of 5 s played,
   **Then** a committed test asserts of the rate log that between each gap's answer and
   the next gap's call every rate equals its axis's feed-forward rate plus the tracking
   offset read just after that answer, within 0.001″/s; that every gap put at least one
   rate for each axis into the log between its call and its answer; and that after
   `tracker.stop("stopped")`, `steered_by_subs()` is false, a `gap` answers (0.0, 0.0)
   at once and the log gains no rate (oracle: 015's own rule that nothing follows a
   stop, `specs/015-tracking/spec.md:229`. A gap before a tracker's first rates and a
   gap nobody waits for are US9-S7 and US9-S8).
8. **Given** the tracker rig with no rate error, a first gap, the error (0.0, 0.0)
   handed over at the time now, a second gap and 6 s waited, **When** the axis
   displacement (−6.0″, 0.0) is handed as an error at the time now and a third gap
   awaited, **Then** a committed test asserts that the azimuth tracking offset is within
   0.001″/s of +0.1 and the altitude tracking offset within 0.002″/s of 0.0 (worked
   example: −6″ of azimuth in 6.0 s is a drift rate of −1.0″/s, under the 1.504″/s of
   the jump test, and 0.3 of it is 0.3″/s; one gap changes an axis's offset by 0.1″/s at
   most. No step lies between the two errors, so the two-deadband rule does not stop
   it).
9. **Given** the tracker rig with no rate error and a first gap, **When** a test reads
   the console time t, hands the tracking error (12.0, −16.0) with the time t − 3.0,
   awaits a second gap, hands (12.0, −16.0) again with the time t − 1.0, and awaits a
   third gap, **Then** a committed test asserts that the second gap's `Stepped` is
   within 0.1″ of (−6.0, 8.0); that the third's is within 0.1″ of (−3.0, 4.0), not
   (−6.0, 8.0): the second error was measured before the first step, which has since
   halved it; and that the tracking offset is exactly `[0.0, 0.0]` after the second gap
   and within 0.002″/s of it after the third (worked example: (12, −16) + (−6, 8) =
   (6, −8), and half of that is taken out; no step lies between the two errors' times,
   so no drift rate was measured but the turn of the sky's axes in 2 s). **And** on
   another rig, after a first gap, the test reads the console time t, hands (12.0,
   −16.0) with the time t − 2.0 and then (6.0, −8.0) with the time t − 1.0, and awaits a
   gap: its `Stepped` is within 0.1″ of (−3.0, 4.0): of two errors handed over between
   two gaps the later is used. **And** on another rig, after a first gap, the test hands
   (12.0, −16.0) with the time t and awaits a gap, then hands (12.0, −16.0) with the
   time t − 1.0 and awaits another: that gap answers exactly (0.0, 0.0): an error no
   later than the last applied is ignored.
10. **Given** the tracker rig with no rate error and a first gap, **When** a test reads
    the console time t, hands the tracking error (0.0, 0.0) with the time t − 20.0 and
    awaits a gap; hands (0.5, 0.0) with the time t − 14.0 and awaits a gap, reading the
    tracking offset o1 after it; and hands (1.0, 0.0) with the time t − 8.0 and awaits a
    gap, reading the tracking offset o2, **Then** a committed test asserts that for each
    axis o1 is within 0.002″/s of −0.3 × d/6, d being that axis's part of a sky offset
    of (0.5, 0.0) by the differencing of
    `crates/finder/tests/motion/tracking/pi_loop.rs:481`; that o2 is within 0.003″/s of
    1.7 × o1 on each axis, and its azimuth part differs from 2 × o1's by more than
    0.006″/s; and that every `Stepped` is exactly (0.0, 0.0) (worked example: the third
    error shows the same drift rate as the second, but it was measured before the gap
    that took 0.3 of that drift rate out, so 0.7 of it is left and 0.3 of that is
    taken: 0.3 + 0.21 = 0.51 of d/6. M42 is east of the meridian at the rig's start, where a place
    to the east has the smaller azimuth: on the azimuth axis d is about −0.66″, so o1
    is about +0.033″/s, 1.7 × o1 +0.056″/s and 2 × o1 +0.066″/s. Both errors are under
    the deadband of two counts, 1.6″: 1.0″ east is about 1.33″ in azimuth and 0.24″ in
    altitude, in size, at M42's place then).
11. **Given** the tracker rig with no rate error, a first `gap`, and eight more gaps,
    each after 6 s of waiting and with no `steer` between them, so that
    `steered_by_subs()` is false after the ninth, **When** a test reads the tracking
    offset o and the truth error e0, calls `tracker.pointing(30.0, 0.0)`, waits 2 s,
    calls it again, waits 1 s and awaits a gap, reading the truth error e1 just after
    that gap's answer, **Then** a committed test asserts that:
    - from the first call to that gap's call every rate in the log equals its axis's
      feed-forward rate within 0.001″/s, o is exactly `[0.0, 0.0]`, and the tracking
      offset read just before the gap's call is exactly o: neither pointing changed a
      rate when it came;
    - the gap answers a `Stepped` within 0.1″ of (−18.0, 0.0), and e1 − e0 is within
      0.5″ of that `Stepped` (worked example: 015's proportional term would have taken
      0.6 of the newest pointing's error out over its interval, and the gap takes it as
      one step, −0.6 × (30.0, 0.0). At M42's place then, 30″ east is about −39.8″ of
      azimuth and −7.3″ of altitude, and both axes turn forward, so the step of about
      +23.9″ and +4.4″ runs with both and is under the 30″ on the sky, 41.0″ of azimuth
      here, that one gap may move an axis);
    - each axis's tracking offset after the gap is exactly o: after a lapse the finder
      camera's pointings step the telescope and change no rate, so the offset stays as
      the subs left it; and the last rate the log holds for each axis at or before that
      gap's answer equals its feed-forward rate plus the tracking offset within 0.01″/s:
      the pulse has ended;
    - `steered_by_subs()` is still false after that gap (oracle: the simulator's truth
      and its horizon transform, differentiated by the test, and 015's constants. A
      held step over one gap's limit is US9-S10, and a noisy finder camera after a
      lapse US9-S11).

---

### User Story 4 - Corrections only in the gap (Priority: P1)

As the stack, before every exposure I tell the console's steering that I am in a gap,
and I start the exposure only when it answers: by then the tracker has made its
correction and the mount has acknowledged its rates. For every sub I accept I hand over
its solve, and when I stop I let the tracker go.

**Why this priority**: A correction during an exposure is a smear. The gap is the only
time one is free.

**Independent Test**: `crates/finder/tests/console/steering_gap.rs`, the module
`steering_gap` of `console`, with the scripted tracker; and, for US4-S5 and US4-S6, which move
the simulated mount, `crates/finder/tests/motion/steering_gap.rs`, the module
`steering_gap` of `motion`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console steering_gap::` and
`cargo nextest run -p finder --locked --test motion steering_gap::`.

**Acceptance Scenarios**:

1. **Given** a scripted steering whose tracker's target is the worked target and whose
   `gap` waits 0.7 s, **When** a test awaits `before_exposure` with the worked ask (a
   wanted exposure of 5 s on the whole sensor at binning 1), calls `accepted` with the
   worked frame
   (`start` 1768528800.0, `exposure_s` 5.0) and the worked solve, and calls `release`,
   **Then** a committed test asserts that:
   - `before_exposure` answered 0.7 s of tokio time after it was called, with
     `exposure_s` 5.0 and `limited` false, and the tracker's log then held exactly one
     `gap`;
   - after `accepted` the log gained exactly one `steer`, with the time 1768528802.5
     and US2-S1's east and north within 0.001″;
   - `release` put one `release` in the log;
   - `view()` was `None` before the first call; after `before_exposure` its `source` is
     `subs` and `subs` 0; after `accepted`, `subs` is 1 and `last` is
     `{"time":1768528802.5,"east_arcsec":…,"north_arcsec":…}` with those numbers; after
     `release`, `source` is `tracker`, `subs` 0 and `last` absent.
2. **Given** a `Steering` whose source answers no tracker, **Then** a committed test
   asserts that `before_exposure` with the worked ask answers with no tokio time passed,
   `exposure_s` 5.0 and `limited` false; and that after `accepted` with the worked frame
   (`start` 1768528800.0, `exposure_s` 5.0) and the worked solve, `view()`'s `source` is
   `none`, its `subs` 0 and its `last` absent.
3. **Given** a scripted steering after one `before_exposure`, **Then** a committed test
   asserts that `accepted` with the worked frame at `start` 1768528700.0, a hundred
   seconds before that call, puts no `steer` in the log and leaves `subs` 0: no exposure
   of this stack holds its middle; that `view()`'s `holding` is false before
   `Steering::hold` is called, true while the `Hold` it answered lives and false once it
   is dropped; that dropping that `Hold` puts one `release` in the log; and that
   `accepted` with the worked frame after it puts no
   `steer` in the log. **And** on another scripted steering whose `gap` waits 0.7 s, a
   `before_exposure` dropped after 0.1 s, then the drop of the `Hold`, leave the log
   holding `gap` and then `release`: the tracker found is released even when no exposure
   was recorded. **And** on another, after one `before_exposure`, `accepted` with the
   worked frame at `start` 1768528799.99, 10 ms before the answer, puts one `steer` in
   the log: an exposure is found by its middle, not its start.
4. **Given** a scripted steering with `set_set_point(Some((668.0, 386.0)))`, **Then** a
   committed test asserts that the `steer` after a `before_exposure` and `accepted` with
   the worked frame and solve carries −54.2373 ± 0.001 east and 95.6174 ± 0.001 north
   (worked example: 50 px right and 25 px up of the centre is 3600 × (−0.0184808°,
   0.0279904°) by the solve's matrix, half of US2-S2's offset, added to US2-S1's answer.
   US2-S2's own set point, (718.0, 361.0), would give a first error 230″ long, which the
   steering stands back from: US4-S8); and that
   with no set point, a `before_exposure` with the worked ask and a frame 200 × 200
   whose `roi` is (400, 200, 400, 400) at binning 2, solved by a solve 200 × 200 with
   the worked solve's centre and twice its matrix, it carries −33.2577 ± 0.001 east and
   −6.2848 ± 0.001 north (worked example: that frame's centre is the sensor's point
   (600, 400), so the sensor's centre, the set point, is (109.0, 105.5) in the frame,
   18 sensor pixels right of and 11 below the frame's centre; 3600 × the worked matrix
   × (18, 11) is (−45.5511, −1.1368), added to US2-S1's answer).
5. **Given** the tracker rig with a rate error of 0.01, built with one change: the
   mount's zero points put its axes on M42's catalogue place at the start, which the
   test computes with `simulator::sky::horizontal` and no refraction, so that the
   simulated imaging frame starts centred on the target; the rig's wait for its first
   rates; and a `Steering` on the rig's clock whose source answers the rig's tracker,
   **When** a test plays 30 subs of 5 s through it late, as 035's worker would (for
   each: await `before_exposure` with the worked ask; read the console time as the
   frame's `start`, and `sim.imaging_truth()`; wait 5 s; read `sim.imaging_truth()`
   again, which also leaves US1's memory the two samples it needs; wait 1 s; and, for
   every sub but the first, call `accepted` for the sub before this one, with the
   worked frame at that sub's `start` and `exposure_s` 5.0 and an `ImagingSolve` built
   with `from_cd` from `sim.imaging_truth_at` for that sub's mid-exposure, its `start`
   plus 2.5 s, as US2-S4 builds one), **Then** a committed test asserts, reading each
   tracking error handed over from `view()`'s `last` right after its `accepted`, that
   the first is at most 5.0″ long; that the first sub's drift, the separation between
   the centres of its two readings, is at least 0.6″; that from the 14th sub on every
   tracking error is at most 3.0″ long and every sub's drift is at most 0.15″; and
   that the rate log holds no rate between a `before_exposure`'s answer and 5 s after
   it that differs from its axis's feed-forward rate plus the tracking offset read at
   that answer by more than 0.001″/s (oracle: the simulator's truth, through US1's
   memory. The rig
   starts on the catalogue place because the simulator draws no refraction: started on
   the apparent place, 65″ higher, the first error would need the altitude axis to
   back up, which no correction may do).
6. **Given** the tracking console before Track is turned on, **Then** a committed test
   asserts that `session.steered()` is `None`; that once the session reports tracking it
   answers a tracker whose `target()` is M42's catalogue place, (83.81867, −5.38967),
   exactly; and that after `POST /api/track` with `{"on":false}` it is `None` again
   (oracle: the session's own Track state).
7. **Given** a console built with `with_imaging`, **Then** a committed test asserts that
   its imaging view has no `steering` key; and that after
   `state.imaging().unwrap().steering()` is given a source that answers no tracker and
   awaits one `before_exposure` with the worked ask, the view's `steering.source` is
   `none` and `steering.subs` 0 (oracle: FR-016).
8. **Given** a scripted steering, **When** a test awaits `before_exposure` with the
   worked ask and calls `accepted` with the worked frame (`start` 1768528800.0) and a
   solve whose tracking error is (200.0, 0.0), `ImagingSolve::from_cd` with the worked
   solve's matrix and size and the centre RA 83.81867° + 200/(3600 × cos 5.38967°), Dec
   −5.38967°, **Then** a committed test asserts that the tracker's log holds `gap` and
   then `release` and no `steer`; that `view()`'s `far_arcsec` is within 0.01 of 200.0,
   its `source` `tracker`, its `subs` 0 and its `last` absent; that a second
   `before_exposure` puts no second `gap` in the log, and `accepted` with the worked
   frame and the worked solve no `steer`; and that after `release`, one more
   `before_exposure` puts a `gap` in the log, `view()` has no `far_arcsec`, and
   `accepted` with the worked frame and a solve built the same way for (100.0, 0.0) puts
   one `steer` in the log and leaves `far_arcsec` absent (oracle: FR-014: the steering
   stands back from a stack whose first tracking error is over 180″, until its next
   `release`, and steers one whose first error is under it).
9. **Given** a `Steering` on a clock that follows tokio's time from 1768528800.0, whose
   source answers a scripted tracker whose `gap` waits 0.7 s, **When** a test awaits one
   `before_exposure` with the worked ask, **Then** a committed test asserts that
   `accepted` with the worked solve and a frame whose `start` is 1768528800.1 and
   `exposure_s` 0.4 puts no `steer` in the log, and that one with the worked frame at
   `start` 1768528800.7 puts one: an exposure is remembered from when `before_exposure`
   answered, not from when it was called (oracle: FR-013).
10. **Given** a scripted steering whose tracker has no target, **When** a test awaits
    `before_exposure` with the worked ask and calls `accepted` with the worked frame
    (`start` 1768528800.0) and the worked solve, **Then** a committed test asserts that
    the log holds no `steer` and that `view()`'s `subs` is 0 (oracle: FR-014).
11. **Given** a scripted steering after one `before_exposure` with the worked ask,
    **When** the test empties the source's cell and drops its own `Arc` of the tracker,
    **Then** a committed test asserts that a `Weak` of it taken before the drop no
    longer upgrades, and that `accepted` with the worked frame and solve leaves `subs`
    0: the steering keeps no tracker alive (oracle: FR-013).
12. **Given** a scripted steering with `set_set_point(Some((668.0, 386.0)))`, after one
    `before_exposure` with the worked ask and one `accepted` with the worked frame
    (`start` 1768528800.0) and the worked solve, so that its log holds one `gap` and one
    `steer` and `view()`'s `subs` is 1, **When** a test calls `set_set_point(None)`,
    **Then** a committed test asserts that the log gains exactly one `release`, and that
    `view()` has `subs` 0 and `last` absent; that after one more `before_exposure`,
    which puts a second `gap` in the log, `accepted` with the same frame and solve puts
    a `steer` in the log that carries US2-S1's east and north within 0.001″; and that a
    second `set_set_point(None)` puts no further `release` in the log (oracle: FR-014. A
    set point that changes lets the tracker go, so that no drift rate is measured across
    the move, and `None` puts it back at the sensor's centre, as 037 asks when a run
    starts its stack).

---

### User Story 5 - The exposure cap (Priority: P2)

As the observer, I choose how many pixels the field may turn at the corner of a sub
during one exposure. A stack then starts with no exposure longer than the tracker allows
for that over the hour ahead, halving the exposure I set as often as needed but never to
under a second, keeps that exposure while it runs, and the readout says when my exposure
was cut, when the limit has since fallen under it, and what a stack started now would
take. I can turn the feedback off, and set the two shares of its control law, for the
field.

**Why this priority**: Field rotation is the one limit no correction removes. It is P2
because short exposures chosen by hand already stay under it.

**Independent Test**: `crates/finder/tests/console/steering_cap.rs`, the module
`steering_cap` of `console`, and, for US5-S6, US5-S8, US5-S11 and US5-S14,
`crates/finder/tests/motion/steering_cap.rs`,
the module `steering_cap` of `motion`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test console steering_cap::` and
`cargo nextest run -p finder --locked --test motion steering_cap::`.

**Acceptance Scenarios**:

1. **Given** a scripted steering whose tracker logs the turn it is asked about and
   answers a longest exposure of 37.15 s, **When** a test awaits `before_exposure` with
   the worked ask (the whole 1236 × 822 sensor at binning 1, a shortest exposure of
   0.001 s) and the wanted exposure each case gives, calling `release` between the
   cases, **Then** a committed test asserts that:
   - the turn asked about equals the test's own `6.0 / 618f64.hypot(411.0)` radians
     within 10⁻¹² (worked example: the corner is hypot(618, 411) = 742.189 px from the
     centre, and the smear is the default, 6 px);
   - a wanted exposure of 60 s answers 30.0 with `limited` true, and 20 s answers 20.0
     with `limited` false;
   - at binning 2 the turn is `6.0 / 309f64.hypot(205.5)`; for the region (400, 200,
     400, 400) at binning 1 it is `6.0 / 218f64.hypot(211.0)`; with the set point
     (718.0, 361.0) on the whole sensor it is `6.0 / 718f64.hypot(461.0)`; and for a
     sensor 1237 × 823, whole and at binning 1, it is `6.0 / 618.5f64.hypot(411.5)`, the
     centre being half the size and not half rounded down; each within 10⁻¹² (worked
     example: the farthest corners are 371.095 px, 303.389 px, 853.256 px and 742.889 px
     from the set point; the figures here are rounded and are not what the test compares
     with);
   - with a longest exposure of 6.0 s, a wanted 60 s answers 3.75; with a longest
     exposure of 8.0 s and a shortest exposure of 10 s it answers 15.0, `limited` true;
     with no longest exposure it answers 60.0, `limited` false; with a longest exposure
     of 30.0 s, a wanted 60 s answers 30.0, `limited` true: an exposure equal to the
     limit is not halved again; and with a longest exposure of 0.7 s, a wanted 5 s
     answers 1.25, `limited` true: the exposure is not halved to under 1 s, where two
     more halvings would have fitted it (no case here has a limit under half the
     exposure it answers: from US6 such a call waits, US6-S17).
2. **Given** the same steering and a first `before_exposure` that answered 30.0 for a
   wanted 60 s, **When** the tracker's longest exposure becomes 25.0 s and a test awaits
   `before_exposure` again without a `release`, for a wanted 60 s and then for a wanted
   10 s, **Then** a committed test asserts that both answer 30.0 with `limited` true:
   the exposure, and whether it was cut when it was decided, are held until `release`,
   whatever is asked for meanwhile; that `view()` has `exposure_s` 30.0, `limited` true,
   `longest_exposure_s` 25.0 and `over_limit` true; that right after `release`, before
   any further call, `view()` has no `exposure_s`, `limited` false and `over_limit`
   false; and that a `before_exposure` for a wanted 60 s then answers 15.0 with
   `limited` true, and `view()` has `over_limit` false; and, on another such steering,
   after its first `before_exposure`, when the tracker's longest exposure becomes
   exactly 30.0 s and `before_exposure` is awaited again, `view()` has
   `longest_exposure_s` 30.0 and `over_limit` false.
3. **Given** the same after `set_smear_px(1.0)`, **Then** a committed test asserts the
   turn asked about equals `1.0 / 618f64.hypot(411.0)` within 10⁻¹², and that
   `set_smear_px` refuses 0.05, 20.5 and a value that is not finite with
   `the corner smear must be a number from 0.1 to 20`.
4. **Given** a scripted steering whose tracker answers a longest exposure of 37.15 s,
   after a first `before_exposure` with the worked ask and a wanted exposure of 60 s,
   **Then** a committed test asserts `view()` has `smear_px` 6.0, `on` true,
   `step_share` 0.5, `rate_share` 0.3, `exposure_s` 30.0, `limited` true, `over_limit`
   false and `longest_exposure_s` 37.15, and after a further `before_exposure` whose
   source answers no tracker, no `longest_exposure_s`: the limit is the running
   tracker's, asked as the view is made, and no tracker is running; and on another,
   whose tracker names no longest exposure, after the same call, `longest_exposure_s` is
   the string `no limit`, `exposure_s` 60.0 and `limited` false.
5. **Given** 032's solving rig (032 spec.md § User Scenarios), **Then** a committed test
   asserts that:
   - `PUT /api/imaging/steering` with `{"smear_px":1.5}` answers 200 with the imaging
     view, whose `steering.smear_px` is 1.5 and `steering.on` true, and
     `GET /api/session`'s `imaging.steering` says the same;
   - `{"on":false}` answers 200 with `steering.on` false and `smear_px` still 1.5,
     `{"step_share":0.25,"rate_share":0}` answers 200 with `steering.step_share` 0.25
     and `steering.rate_share` 0.0, and `{}` answers 200 and changes nothing;
   - `{"smear_px":0}`, `{"smear_px":"2"}`, `{"on":1}`, `{"smear":2}`,
     `{"step_share":1.5}` and `{"rate_share":-0.1}` each answer 400 with a JSON `error`,
     the first exactly `the corner smear must be a number from 0.1 to 20`, the fifth
     exactly `the step share must be a number from 0 to 1` and the sixth exactly
     `the rate share must be a number from 0 to 1`, and change nothing; after
     `{"on":false}`, `{"on":true,"smear_px":0}` answers 400
     and `on` is still false; and `{"on":null}` and `{"smear_px":null}` each answer 400;
   - `{"on":true}` with a foreign `Origin` answers 403; the same body sent with
     `Content-Type: text/plain` answers 415; the body `not json` sent as
     `application/json` answers 400; and none of the three changes anything: `on` is
     still false;
   - a console built the same way without `with_imaging` answers 409 with
     `this console takes no imaging frames`.
6. **Given** the tracker rig with no rate error, **Then** a committed test asserts that
   `tracker.longest_exposure_s(2.0 / 618f64.hypot(411.0))` equals, within one part in a
   million, the seconds `ephemeris::longest_exposure(23.63232, 15.71664, 19.12, r, 2.0)`
   answers (`crates/finder/src/ephemeris.rs:105`), r being the field rotation the test
   computes for M42 with `sky::alt_az` and `ephemeris::altaz_rates_from_alt_az`, as
   `crates/finder/tests/motion/tracking/routes.rs:312` does, in radians a second, at
   the console time it reads immediately before the call with no await between the
   two; and that it is between 30 s and 45 s (worked example: 1236 × 822 pixels of
   19.12 µm are 23.63232 × 15.71664 mm; at 14.91°/h the answer is 37.28 s). The same
   holds, with the same oracle, for M81 (`targets::find("M81")`), whose field rotation
   at the rig's start is negative; through `&dyn Steered` the answer is the same;
   `longest_exposure_s` answers `None` for a turn of 0.0, −1.0 and NaN; and after
   `stop("stopped")` it answers `None`.
7. **Given** a scripted steering whose tracker answers a longest exposure of 37.15 s,
   with `set_on(false)`, **When** a test awaits `before_exposure` with the worked ask
   and a wanted 60 s and calls `accepted` with the worked frame and solve, **Then** a
   committed test asserts that the log holds no `gap`, no `tune` and no `steer`, that
   the call answered 30.0 with `limited` true, and that `view()`'s `source` is `tracker`
   and its `on` false: the switch turns off the steering and not the cap. **And** on a
   steering never used, `set_on(false)` alone makes `view()` answer, with `on` false.
8. **Given** a tracker rig whose target is due east at an instant the test chooses, t* =
   1768528830.0, 30 s after the rig's clock starts: with Dec fixed at +18.75° the test
   steps RA through 0°, 1°, … 359°, takes the first two neighbours whose azimuths at t*
   by `sky::alt_az` for 40° N 75° W lie either side of 90° with both altitudes above 0°,
   and bisects RA between them 60 times, so that the azimuth at t* is 90° within 10⁻⁹
   degrees (the altitude is then near 30°); the rig's target is a clone of M42's
   `Target` with that `ra_deg` and `dec_deg`, and its mount's zero points put its axes
   on that place's apparent direction at the start, as the tracker rig does for M42;
   **When** the test sleeps until the rig's clock reads t* to within 2 ms (tokio's timer
   counts in milliseconds) and calls `longest_exposure_s(0.0027)` with no await after
   the sleep, **Then** a committed test asserts it is `None`; and that after 10 s more
   it is `Some` of more than 3600 s (oracle: 015's own function and its 10⁻⁹ rad/s rule.
   Worked example: due east at this place the azimuth moves 9.7″ a second, and the field
   rotation passes 10⁻⁹ rad/s 3.2″ of azimuth from due east, a third of a second either
   side of t*, so the instant matters and 2 ms is 0.02″; 10 s later the azimuth is 97″
   past east, the rotation 3 × 10⁻⁸ rad/s and the answer about 89,000 s).
9. **Given** a scripted steering after one `before_exposure` with the worked ask and one
   `accepted` with the worked frame (`start` 1768528800.0) and the worked solve, so that
   its log holds one `gap` and one `steer` and `view()` has `source` `subs` and `subs`
   1, **When** a test calls `set_on(false)`, **Then** a committed test asserts that the
   log gains exactly one `release`; that `view()` has `source` `tracker`, `subs` 0, no
   `last` and `exposure_s` still 5.0; that a second `before_exposure` puts no second
   `gap` in the log and answers 5.0; and that after `set_on(true)` a third puts one
   (oracle: FR-019. Turned off while a stack runs, the switch lets the tracker go at
   once, so that its own corrections steer; as drafted the claim stood, no gap came to
   lapse it, and nothing steered).
10. **Given** a scripted steering whose tracker answers a longest exposure of 37.15 s
    and, for the hour ahead, of 20.0 s, **When** a test awaits `before_exposure` with
    the worked ask and a wanted 60 s, **Then** a committed test asserts that it answers
    15.0 with `limited` true, and that `view()` has `exposure_s` 15.0,
    `longest_exposure_s` 37.15 and `over_limit` false: the exposure is decided from the
    shortest limit of the hour ahead and held, and the view shows the limit as it is now
    (oracle: FR-018; worked example: 60 s halved twice is 15 s, the first value not over
    20 s).
11. **Given** the tracker rig with no rate error, **Then** a committed test asserts that
    `tracker.longest_exposure_ahead_s(2.0 / 618f64.hypot(411.0))` equals, within one
    part in a million, the smallest of the 61 values `turn / r.abs()` the test computes,
    r as US5-S6 computes it, for M42 at the console time it reads immediately before the
    call, with no await between the two, and at each whole minute after that time up to
    the sixtieth; that it is less than `tracker.longest_exposure_s` of the same turn
    asked at that instant, and between 33 s and 36 s; that through `&dyn Steered` the
    answer is the same; that it is `None` for a turn of 0.0, −1.0 and NaN; and that
    after `stop("stopped")` it is `None` (oracle: 015's own functions, sampled by the
    test as 015 samples its forecasts. Worked example: the rig starts 54 minutes before
    M42's transit, where the field turns fastest, 16.19°/h against 14.91°/h at the
    start, so the shortest limit of the hour is the transit's: 34.33 s for this turn,
    against 37.28 s at the start).
12. **Given** a scripted steering never used, whose tracker answers a longest exposure
    of 37.15 s and, for the hour ahead, of 20.0 s, **Then** a committed test asserts
    that `view()` is `None`; that `view_for(Some(ask))`, the ask being the worked ask
    with a wanted 60 s, has `source` `tracker`, `longest_exposure_s` 37.15,
    `would_take_s` 15.0, no `exposure_s` and `limited` false, and leaves `view()`
    `None`; that when the tracker names no longest exposure it has `longest_exposure_s`
    `no limit` and `would_take_s` 60.0; that when the source answers no tracker
    `view_for(Some(ask))` is `None`; and that after a `before_exposure` for a wanted
    60 s that found the first tracker, `view_for` with the worked ask and a wanted 10 s
    has `exposure_s` 15.0 and `would_take_s` 10.0: what a stack started now would take
    is worked from the ask given, whatever a running stack holds. **And** on 032's
    solving rig, with the console's steering given a source that answers a scripted
    tracker naming no longest exposure and no other call made, `GET /api/imaging`'s
    `steering.longest_exposure_s` is `no limit` and its `steering.would_take_s` equals
    that view's `settings.exposure_ms` over 1000: the console asks with its own camera
    and settings (oracle: FR-018).
13. **Given** a scripted steering, **When** a test awaits `before_exposure` with the
    worked ask, calls `set_step_share(0.25)` and `set_rate_share(0.0)`, and awaits
    `before_exposure` again, **Then** a committed test asserts that the tracker's log
    holds `tune(0.5, 0.3)` right before its first `gap` and `tune(0.25, 0.0)` right
    before its second; that `view()` has `step_share` 0.25 and `rate_share` 0.0; that
    `set_step_share` refuses −0.1, 1.5 and a value that is not finite with
    `the step share must be a number from 0 to 1`, and `set_rate_share` the same three
    with `the rate share must be a number from 0 to 1`, each leaving the view as it was;
    and that both accept 0.0 and 1.0 (oracle: FR-030).
14. **Given** the tracker rig with no rate error and a first gap, **When** a test calls
    `tracker.tune(1.0, 0.6)`, hands the error (0.0, 0.0) at the time now, awaits a gap,
    waits 6 s, hands (0.5, 0.0) at the time now and awaits a gap, reading the tracking
    offset o after it, waits 6 s more, hands (12.0, −16.0) at the time now and awaits
    one more gap, **Then** a committed test asserts that for each axis o is within
    0.002″/s of −0.6 × d/6, d being that axis's part of a sky offset of (0.5, 0.0) by
    the differencing of `crates/finder/tests/motion/tracking/pi_loop.rs:481`, and that
    its azimuth part is above +0.05″/s; that the last gap's `Stepped` is within 0.2″ of
    (−12.0, 16.0); and that on another rig, after `tracker.tune(0.5, 0.0)`, the same
    first two errors leave the tracking offset exactly `[0.0, 0.0]` (oracle: the
    simulator's truth and its horizon transform, differentiated by the test. Worked
    example: 0.5″ east is about −0.66″ of azimuth, under the deadband, so no step lies
    between the first two errors; with a rate share of 0.6 the azimuth offset is about
    +0.066″/s, where the default 0.3 gives +0.033″/s; and with a step share of 1.0 the
    whole of (12, −16) is stepped out, about 10.4″ and 18.5″ of axis angle, which runs
    with both axes and is under one gap's limit).

---

### User Story 6 - Not steady: pause, and drop what was taken across it (Priority: P2)

As the stack, I start no exposure while the tracker says it is not steady: during a
backlash take-up, a first reversal that is still being measured, or the keyhole pass;
while a hand is moving the telescope; for the 2 s the tube is given to settle after a
take-up, the keyhole pass and a tracker's first rates; and in a turn that finds an axis
stopped. A sub whose exposure crossed such a time is dropped before it is solved, with
that reason. While it is not steady the tracker corrects itself as 015 has it, even
after the subs have lapsed, so that a first reversal's measurement still closes. I also
start none while the tracker's limit is under half the exposure I hold: I let the
tracker go and wait for the limit to rise.

The cases of the reanalysis's second pass took this story's estimate past 85% of the
64 KiB budget, so it was cut as User Story 3 was (the owner's choice 14 of 2026-11-19):
eight scenarios of User Story 10 are US6-S11 whole and the later halves of US6-S1 and
US6-S2, which each held more than one Given, When and Then. What they test is still this
story's code. The scenario that was US6-S12 took the number 11, and the later halves of
US6-S3 are US6-S12 to US6-S16 (§ Clarifications, the session of the reanalysis).

**Why this priority**: Altitude reverses at transit, where a target is highest and a
run will sit. It is P2 because it is rare within a night.

**Independent Test**: `crates/finder/tests/motion/steering_steady.rs`, the module
`steering_steady` of `motion`, on the backlash rig and, for US6-S6, on
`crate::support::goto::Rig`; and `crates/finder/tests/console/steering_steady.rs`, the
module `steering_steady` of `console`, with the scripted tracker. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs both; during
development, `cargo nextest run -p finder --locked --test motion steering_steady::` and
`cargo nextest run -p finder --locked --test console steering_steady::`.

**Acceptance Scenarios**:

1. **Given** the backlash rig, followed to 60 s before the transit with
   `rig.follow(transit - 60.0, 60.0)`, and one `gap` awaited then, **When** a test runs
   three futures to their ends with `tokio::join!` on its own task, each borrowing the
   rig (the rig holds its `Tracker` by value, so nothing can be spawned over it): the
   follower, `rig.follow(transit + 120.0, 60.0)`; the reader, which every 50 ms until
   the rig's clock passes transit + 120 s notes the console time,
   `rig.tracker.steady()`, `rig.tracker.pause()` and `rig.tracker.steered_by_subs()`;
   and the asker, which looks every 10 ms, until the rig's clock passes transit + 120 s,
   for the reader's first false reading of `steady()`, then awaits
   `rig.tracker.gap()` once and notes the console time of its answer, **Then** a
   committed test asserts, with F and L the times of the reader's first and last
   false readings, that:
   - `steady()` read false at least once, every reading from F to L is false with
     `pause()` `Some("reversing")`, and the reading 1 s before F has `pause()` `None`;
   - the altitude rate of magnitude 0.5°/s in the rate log, and the zero rate that ends
     it, both lie between F − 0.05 s and L + 0.05 s (oracle: the rig's rate log, as
     `crates/finder/tests/support/tracker.rs:285` reads it);
   - `steady_between(F - 20.0, F - 1.0)` and `steady_between(L + 1.0, L + 20.0)` are
     true, and `steady_between(F - 5.0, F + 0.1)`, `steady_between(L - 0.1, L + 5.0)`
     and `steady_between(F - 5.0, L + 5.0)` are false, all asked once the three
     futures have ended;
   - the reader's `steered_by_subs()` is true at its reading 1 s before F and false at
     L;
   - the asker's `gap` answers exactly (0.0, 0.0), no earlier than the zero rate that
     ends the take-up and at least 1.5 s before L: a gap asked for during a take-up is
     served once the take-up's turn is over, never inside it, and is turned away then,
     because the tracker settles for 2 s more (oracle: the rig's rate log and clock.
     What the mount is left running at the first gap after the settling is US10-S1).
2. **Given** `crate::support::tracker::Rig` for M42 from 120 s before its transit on a
   fresh session, so that no backlash is measured, **When** a test joins
   `rig.follow(transit + 120.0, 60.0)` with a reader as in US6-S1, **Then** a committed
   test asserts, with Z the time of the altitude axis's first zero rate in the rate
   log, that every reading of `steady()` from 3.5 s after the rig's start to Z − 0.1 s
   is true, that the reading nearest Z + 5 s is false, with `pause()`
   `Some("measuring")`, and that `steady_between(Z + 0.1, Z + 10.0)`, asked at the end,
   is false (oracle: the rig's rate log and clock. The measurement's close, a start and
   a stop, what a take-up forgets, the trait and the keyhole pass are US10-S2 to
   US10-S7).
3. **Given** a scripted steering whose tracker says it is not steady, with the `pause`
   word `reversing`, and whose `gap` waits 0.1 s, **When** a test awaits
   `before_exposure` with the worked ask in a task,
   waits 10.1 s, sets the tracker steady and waits 1 s, **Then** a committed test
   asserts that the call had not answered after the 10.1 s, with `view()`'s `steady`
   false, `waiting` true, `pause` `reversing`, `Steering::waiting()` true and no `gap`
   in the log; and that it answered within 0.5 s of the change, after exactly one `gap`,
   with `steady` true, `waiting` false, no `pause` and `Steering::waiting()` false (the
   five later cases this scenario held are US6-S12 to US6-S16).
4. **Given** a scripted steering after a `before_exposure` with the worked ask, whose
   tracker answers `steady_between` false, **Then** a committed test asserts that
   `check` with the
   worked frame (`start` 1768528800.0, `exposure_s` 5.0) answers
   `Verdict::Drop("not_steady")`, and that `view()`'s `dropped_not_steady` is 1, and, on
   another such steering after the same two calls, that after `release` it is still 1
   and after a new `Steering::hold` it is 0: the count is one stack's, and lasts until
   the next stack begins; that with `steady_between` true it answers `Verdict::Keep` and
   the count stays 1; that the tracker's log shows the window it was asked about as
   (1768528800.0, 1768528805.0); that after the test has made the source answer no
   tracker and dropped its last `Arc` of the scripted one, `check` with the same frame
   answers `Drop("not_steady")`: a tracker that has gone was not steady; and that on a
   `Steering` whose source answers no tracker, `check` after a `before_exposure` answers
   `Keep`; and, on another scripted steering as in the first case, that `check` with the
   worked frame started 0.01 s earlier (`start` 1768528799.99) also answers
   `Drop("not_steady")`: the record is found by the frame's mid-exposure, not its start;
   and, on another such steering after `set_on(false)` and a `before_exposure` with the
   worked ask, that `check` with the worked frame answers `Drop("not_steady")` too: the
   switch turns off the steering, not the drop.
5. **Given** `crate::support::tracker::Rig` for M42 from 600 s before its transit on a
   fresh session, as `crates/finder/tests/support/tracker.rs:326` builds one, so that
   no backlash is measured; a first `gap` awaited once `steady()` reads true (read every
   10 ms, for at most 5 s: the tracker settles for 2 s after its first rates); and eight
   more gaps, each after 6 s of waiting and with no `steer`, so that the claim has
   lapsed, **When** a test runs with `tokio::join!` on its own task the follower,
   `rig.follow(transit + 600.0, 60.0)`, and an asker that, until the rig's clock
   passes transit + 600 s, waits 6 s, awaits `rig.tracker.gap()` and notes
   `rig.tracker.steady()` and `rig.tracker.steered_by_subs()` just after its answer,
   **Then** a committed test asserts that the follower's own bound held (it asserts
   the true error is at most 60″ at every step, `crates/finder/tests/support/tracker.rs:269`);
   that `GET /api/session`'s `backlash.alt_deg` is within 30″ of 0.3°, as
   `crates/finder/tests/support/tracker.rs:332` asserts after the same follow with no
   stack; that at least one of the asker's notes of `steady()` is false and the last
   is true; that every note of `steered_by_subs()` is false; and that `steady()` is
   true once the two futures have ended (oracle: the mount's configured 0.3° of play
   and 015's own bounds. While the first reversal's measurement is open the tracker is
   not steady, its gaps are turned away, and the finder camera's pointings correct it
   as 015 has them though the claim had lapsed; held for a gap instead, they would
   never be applied, and the measurement would not close).
6. **Given** the tracking console, its session's tracker taken once
   `steady()` reads true (read every 100 ms, for at most 10 s) and one `gap` awaited, so
   that `steered_by_subs()` is true, **When** a test pushes the tube with
   `rig.mount.push_by(0.0, -0.1)`, 6′ as `crates/finder/tests/motion/tracking/push.rs`
   pushes it, and from then on reads `GET /api/session` and the tracker every 100 ms;
   puts the tube back with `rig.mount.push_by(0.0, 0.1)` at the first read whose
   `track.push.state` is `moving`; and, once `track.push` is absent again, pushes it a
   second time with `rig.mount.push_by(0.0, -0.1)` and leaves it there, **Then** a
   committed test asserts that:
   - at the first read whose `track.push.state` is `moving`, `steady()` is false,
     `pause()` is `Some("pushed")` and `steered_by_subs()` is false, and a `gap` awaited
     then answers exactly (0.0, 0.0) and leaves `steered_by_subs()` false;
   - at the first read after it that has no `track.push`, `steady()` is true, and a
     `gap` awaited then makes `steered_by_subs()` true: a push that comes back ends the
     time not steady with no follow;
   - after the second push, with m the console time of the first read whose
     `track.push.state` is `moving` and f that of the first whose state is `following`,
     `steady()` is false at m and true at f, `steady_between(m, f)`, asked at f, is
     false, and a `gap` awaited at f makes `steered_by_subs()` true (oracle: 038's own
     `track.push`, as its tests read it. While the session reads a push as moving the
     tracker is not steady: the claim is dropped, no gap steps the telescope back toward
     the old place, and the stack waits).
7. **Given** the backlash rig followed to 60 s before the transit with
   `rig.follow(transit - 60.0, 60.0)` and one `gap` awaited then, so that
   `steered_by_subs()` is true, **When** a test waits until the rate log gains an
   azimuth rate, at the time r, waits 0.5 s, stops both axes on the simulated mount
   itself with `rig.mount.stop_all().await`, and waits 2 s, **Then** a committed test
   asserts that `steady_between(r + 0.1, r + 0.9)` is false; that
   `steady_between(r - 5.0, r - 0.1)` and `steady_between(r + 1.1, r + 1.9)` are true;
   that `steady()` is true and `steered_by_subs()` false at the end; and that the first
   azimuth rate the log holds after r is not zero (oracle: the rig's rate log and clock.
   Stopping the mount behind the tracker's back is what the scope node does when a rate
   command fails on the mount's side: both axes stop and the command is still
   acknowledged. The tracker's next turn finds the azimuth axis at rest although the
   rate it last sent was not zero, reads not steady from the time of that rate to its
   own next rates, which start the axes again, and so drops the claim).
8. **Given** the lapsed backlash rig and a `pointing(0.0, 0.0)` right after its ninth
   gap, **When** a `pointing(30.0, 0.0)` arrives once the take-up at transit has made
   `steady()` false, a `pointing(0.0, 0.0)` arrives as soon as `steady()` reads true
   again, both read every 10 ms, and a `gap` is awaited at once, **Then** a committed
   test asserts that the azimuth rate in force when the gap answers, the last in the
   rate log at or before its answer, is the feed-forward rate plus the tracking offset
   read just after the answer, within 0.01″/s, and so is the first rate after it: a
   pointing inside 015's deadband that arrives steady and lapsed zeroes the proportional
   rate in the tracker's own state and sends nothing, so only a gap that sends whatever
   that state says takes the rate off the mount (oracle: the rig's rate log. The recheck
   of round 1 measured +25.3″/s still in force at such a gap's answer, for a pointing
   with no earlier one and no settling).
9. **Given** the lapsed backlash rig, with b the console time read right after its ninth
   gap, **When** the take-up at transit has made `steady()` false and a test hands over
   a `steer` of (12.0, −16.0) at the time now; and, once `steady()` reads true again,
   both read every 10 ms, it hands over a `steer` of (12.0, −16.0) with the time b and
   awaits a `gap`, **Then** a committed test asserts that the gap answers exactly (0.0,
   0.0) and leaves `steered_by_subs()` false: a tracking error handed over while the
   tracker is not steady, or measured before it stopped being steady, steers nothing and
   takes no lapsed tracker back; and that a `steer` of (0.0, 0.0) at the time now and
   one more gap then make `steered_by_subs()` true (oracle: FR-011 and FR-021).
10. **Given** the backlash rig followed to 3 s before the transit with
    `rig.follow(transit - 3.0, 60.0)`, **When** a test reads `steady()` every 10 ms
    until it is false, for at most 5 s, and then joins, on its own task, an asker that
    awaits `rig.tracker.gap()` under a timeout of 1.1 s and a stopper that waits 0.1 s
    and calls `rig.tracker.stop("stopped")`, **Then** a committed test asserts that the
    gap answered, exactly (0.0, 0.0): the take-up holds the rate task for most of a
    second, the gap waits in the tracker's queue, and tracking that ends answers every
    gap still waiting (oracle: 015's Stop; without it a stack's taker would wait in
    `before_exposure` for ever).
11. **Given** a scripted steering whose tracker is steady and whose `gap` waits 5 s,
    **When** a test awaits `before_exposure` with the worked ask in a task, sets the
    tracker not steady 0.5 s later and waits 0.6 s more, **Then** a committed test
    asserts that `view()` has `steady` false and `waiting` true, that
    `Steering::waiting()` is true, and that the log holds exactly one `gap`: a tracker
    that stops being steady while its gap is awaited, as one does whose keyhole pass
    begins just then, shows as a wait.
12. **Given** a scripted steering whose tracker's `gap` waits 1 s, **When** a test
    awaits `before_exposure` with the worked ask in a task, sets the tracker not steady
    0.5 s later and steady again 5 s after that, **Then** a committed test asserts that
    the call has not answered 4 s after it was made, and that it answers between 6.4 s
    and 6.8 s after it was made, with exactly two `gap`s in the log: the first gap ended
    on a tracker that was not steady, so it does not count.
13. **Given** a scripted steering whose tracker is steady, after one `before_exposure`,
    and a `Steering` whose source answers no tracker, after one, **Then** a committed
    test asserts that each `view()` has `steady` true, `waiting` false, no `pause` and
    `dropped_not_steady` 0.
14. **Given** a scripted steering whose tracker is not steady, a `Hold` taken from it
    and a `before_exposure` awaited in a task for 2 s, **When** the test aborts that
    task and drops the `Hold`, **Then** a committed test asserts that
    `Steering::waiting()` is false and that `view()` has `waiting` false, `steady` true
    and no `pause`: a wait abandoned with its stack does not go on showing.
15. **Given** a scripted steering whose tracker is not steady, after `set_on(false)`,
    **When** a test awaits `before_exposure` with the worked ask in a task, **Then** a
    committed test asserts that it has not answered 1 s after it was called, with
    `steady` false and `waiting` true, and that once the tracker is steady it answers
    with no `gap` in the log: the switch turns off the steering, not the wait. **And**
    on another such steering with the switch still on, where `set_on(false)` is called
    0.5 s into the wait, the same holds: the switch is read where a gap would be asked
    for, not when the call is made.
16. **Given** a scripted steering whose tracker is not steady and a `before_exposure`
    waiting in a task for 1.1 s, **When** the test makes the source answer no tracker,
    **Then** a committed test asserts that the call answers within 0.25 s, and that
    `view()` has `steady` true and `waiting` false.
17. **Given** a scripted steering whose tracker answers a longest exposure of 37.15 s,
    after a first `before_exposure` with the worked ask and a wanted 60 s that answered
    30.0, **When** the tracker's longest exposure becomes 14.0 s and a test awaits
    `before_exposure` again in a task, waits 1.1 s, sets the longest exposure to 15.0 s
    and waits 0.5 s, **Then** a committed test asserts that the call had not answered
    after the 1.1 s, with `view()`'s `waiting` true, `steady` true, `pause` `limit` and
    `over_limit` true, `Steering::waiting()` true, and the tracker's log holding one
    more `release` and no second `gap`; and that it answered within 0.5 s of the change,
    30.0 with `limited` true, after a second `gap`, with `waiting` false and no `pause`
    (oracle: FR-022. While the limit is under half the exposure a stack holds, its subs
    would be smeared by more than twice the smear chosen, so it takes none, lets the
    tracker go to its own corrections and goes on by itself once the limit has risen:
    15.0 s is half of 30.0 s and is not under it).

---

### User Story 7 - The stack asks before every exposure (Priority: P2)

As the observer with Track on and a stack running, I get subs whose stars are round:
the stack's taker asks the steering before every exposure and uses the exposure it
allows, its worker drops a sub the tracker was not steady for and hands over every sub
it accepts, and the tracker is let go when the stack stops.

**Why this priority**: It is what makes the earlier stories reach a picture. It is P2
because it consumes them, and it is the one story that edits 035's stack.

**Independent Test**: `crates/finder/tests/motion/steering_stack.rs`, the module
`steering_stack` of `motion`. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test motion steering_stack::`.

**The stacking rig** is 032's solving rig with these changes: the mount is
`SimMount::with_config` with `MountConfig { base_tilt_deg: 0.0, base_tilt_toward_deg:
0.0, zero_points_deg: [120.0, 50.0], backlash_deg: 0.0 }`, the azimuth and altitude of
the rig's `push_to(50.0, 120.0)`, connected before that push; the session has the
target M42 and the aim point (700.0, 420.0), the true aim point; the telescope is
pointed with `sim.point_eyepiece_at` at M42's catalogue place; the test waits until the
pointing model has a sync and the session's pointing is within 1′ of M42, as
`crates/finder/tests/motion/tracking/routes.rs:96` does, and turns Track on with
`POST /api/track`; then it waits until the session's `track.state` is `tracking` and the
imaging view's `state` is `ready`. (Round 1's rig waited 1 s more here, D38. It no
longer needs to: the tracker settles for 2 s after its first rates, the stack's first
`before_exposure` waits that out, and a sub that starts at the instant the tracker
became steady is kept: plan.md § The tracker's two answers.) **Waiting until** a
condition means reading `GET /api/imaging`, or `GET /api/session` for the session's
parts, every 100 ms until it holds, for at most 300 s of virtual time, and failing
otherwise. **A saved sub** is one whose FITS file is in the stack's folder
`runs/<id>/subs/`, `<id>` being the `stack.id` that `POST /api/imaging/stack` answers;
subs are numbered in the order of their file names, which are their imaging frame ids.
**A sub's drift** is the separation between `sim.imaging_truth_at` at the sub's start
and at its start plus its exposure, both read from that file: its `DATE-OBS`, UTC to the
millisecond, and its `EXPTIME`, in seconds. (031's `frame` part is not used: the taker
takes the next sub at the instant one is saved, so a read every 100 ms sees few of them
`saved`.) **The stack's counts** are the `stack` part of the imaging view (035); **Σ
dropped** is the sum of the twelve counts of its `dropped`.

**Acceptance Scenarios**:

1. **Given** the stacking rig with `set_rate_error(0.01)` on its mount and
   `PUT /api/imaging/settings` with `{"exposure_ms":5000}`, **When** a test starts a
   stack with `POST /api/imaging/stack` and waits until `steering.subs` is at least 18,
   **Then** a committed test asserts that `steering.source` is `subs`; that every
   sub's drift from the 14th sub on is at most 0.15″; that `stack.dropped.not_steady`
   is 0; and that after `DELETE /api/imaging/stack`, `steering.source` is `tracker` and
   `steering.subs` 0 (oracle: the simulator's truth. The same rig with the switch off is
   US7-S8).
2. **Given** the stacking rig, `PUT /api/imaging/steering` with `{"smear_px":2}` and
   `PUT /api/imaging/settings` with `{"exposure_ms":60000}`, **When** a test reads
   `GET /api/imaging`, starts a stack and waits until its first sub is saved, **Then** a
   committed test asserts that the read before the start has `steering.would_take_s`
   30.0 and no `steering.exposure_s`; that the sub's `EXPTIME` is 30.0; that
   `settings.exposure_ms` is 30000.0 while the stack runs; that `steering.limited` is
   true, `steering.exposure_s` 30.0, `steering.over_limit` false and
   `steering.longest_exposure_s` within 2% of the figure the test computes as US5-S6
   does for the console time of its read; and that `settings.exposure_ms` is 60000.0
   once `DELETE /api/imaging/stack` has answered (the stack set the exposure through its
   own lease and put the observer's back through it before letting the camera go. Worked
   example: at the default smear of 6 px the rig's limit is about 112 s and a setting of
   60 s is not cut, so the scenario sets 2 px; the limit is then 37 s now and 34.3 s at
   its shortest in the hour ahead, at M42's transit, and 60 s is halved once).
3. **Given** the stacking rig with `{"exposure_ms":5000}`, a stack running and
   `stack.stacked` at least 2, **When** a test waits until an imaging frame is
   `exposing`, sends `POST /api/track` with `{"on":false}` and then `{"on":true}`,
   waits until `stack.taken` has risen by 3 more and stops the stack, **Then** a
   committed test asserts `stack.dropped.not_steady` is at least 1,
   `steering.dropped_not_steady` equals it, and `stack.taken` equals `stack.stacked`
   plus Σ dropped; and that `stack.json`'s `subs` lists as many subs with `outcome`
   `not_steady` as `stack.dropped.not_steady`, and that
   `Dropped { not_steady: 2, ..Default::default() }.total()` is 2.
4. **Given** 032's solving rig as it stands, with no target and Track off, and
   `{"exposure_ms":2000}`, **When** a test starts a stack, waits until `stack.taken` is
   at least 3 and stops it, **Then** a committed test asserts that `steering.source` is
   `none` and `steering.subs` 0 throughout, that every saved sub's `EXPTIME` is 2.0, and
   that `stack.dropped.not_steady` is 0: with no tracker the stack is
   neither capped nor paused, and nothing is dropped for the tracker's sake.
5. **Given** 032's solving rig as it stands, `{"exposure_ms":60000}`, and the console's
   steering given a source of the test's own with
   `state.imaging().unwrap().steering().set_source(…)`: a scripted tracker whose target
   is the worked target, which names a longest exposure of 37.15 s and whose `gap`
   waits 10 s, **When** a test starts a stack, calls the simulator's
   `set_unresponsive(true)` (`crates/simulator/src/controls.rs:115`) 1 s later, while
   the stack's first `before_exposure` waits in that gap, and waits until
   `stack.state` is `ended`, **Then** a committed test asserts that
   `stack.ended_reason` is `camera`; that `stack.error` is exactly
   `the exposure could not be set to 30000 ms: the scope node is not connected`; that
   `stack.taken` is 0; that the scripted tracker's log holds exactly one `gap` and,
   after it, one `release`; and that, after `set_unresponsive(false)` and a wait until
   the imaging view's `state` is `ready`, `settings.exposure_ms` is 60000.0 (oracle:
   031's own words for a camera that is not ready, which the stack's lease passes on;
   the link is lost within 3 s by 031 US1-S6, well inside the gap's 10 s. A stack that
   cannot shorten its exposure takes no sub at the observer's. A stop while the first
   gap waits, and a link lost while the second waits, are US7-S9 and US7-S10).
6. **Given** the text of `crates/finder/src/imaging/stack/service.rs`, 035's file as
   this story leaves it, read by the test from `CARGO_MANIFEST_DIR`, **Then** a
   committed test asserts it holds none of `ephemeris`, `alt_az`, `parallactic`,
   `counts_per_revolution`, `AxisInfo`, `MountClient`, `azimuth`, `altitude`,
   `field_rotation`, `hour_angle`, `meridian`, `backlash` and `keyhole` (oracle: the
   plan document's rule 1, read as US2-S6 reads it of `steering.rs`: the stack reaches
   the tracker only through `Steering`, and the calls this story adds to it say nothing
   of axes).
7. **Given** the stacking rig with `{"exposure_ms":5000}` and one change, made right
   after its `sim.point_eyepiece_at`: the test reads `c = sim.telescope().camera`, calls
   `sim.set_optics` with `Optics { eyepiece_px: (716.0, 420.0), ..Optics::default() }`,
   and calls `sim.push_to(c.alt, c.az)`, so that the finder camera still has M42 at the
   session's aim point, (700.0, 420.0), while the eyepiece and the imaging camera look
   16 finder pixels from it, about 8′ (the simulated finder camera's 12° across 1456 px
   are 29.8″ a pixel): the session's aim point is that far from the true one, **When** a
   test starts a stack, reads `GET /api/session`, `GET /api/imaging` and
   `sim.telescope().eyepiece` every 100 ms until `stack.taken` is at least 12, and stops
   the stack, **Then** a committed test asserts that at the last read before the stop
   `steering.far_arcsec` is between 420 and 540, `steering.source` is `tracker` and
   `steering.subs` is 0; that no read of the session has a `track.push`; and that the RA
   and Dec of every reading of the eyepiece are within 60″ of the first (oracle: the
   simulator's truth and 038's own `track.push`. Worked example: 16 px × 29.8″ is 477″,
   over the 180″ beyond which the steering stands back. Steered, the subs would have
   stepped the telescope 30″ a gap toward centring M42, and once the finder camera's
   pointing was 5′ from the target 038 would have read a push nobody made).
8. **Given** the stacking rig with `set_rate_error(0.01)` on its mount and
   `PUT /api/imaging/settings` with `{"exposure_ms":5000}`, **When** a test calls
   `sim.stop_frames()` (`crates/simulator/src/controls.rs:106`) as soon as the session
   reports `tracking`, so that no finder pointing reaches the tracker and its own
   corrections change no rate, sends `PUT /api/imaging/steering` with `{"on":false}`,
   starts a stack and waits until 12 subs are saved, **Then** a committed test asserts
   that `steering.subs` stays 0, that `steering.source` is `tracker`, and that at least
   8 of the 12 subs drift more than 0.5″ (oracle: the simulator's truth; worked example:
   with nothing correcting it, a mount 1% fast drifts 0.749″ in a 5 s sub. The finder
   camera's frames are stopped because 015's own loop would otherwise start to correct
   once the error passed its 10″ deadband, about 67 s after Track began, and the bound
   would then rest on how long the rig's waits took).
9. **Given** 032's solving rig as it stands, `{"exposure_ms":60000}`, and the console's
   steering given US7-S5's scripted tracker as its source, **When** a test starts a
   stack and sends `DELETE /api/imaging/stack` 1 s into the first gap, **Then** a
   committed test asserts that the `DELETE` answers within 0.5 s of virtual time, that
   the stack ends `stopped` with `taken` 0, and that the tracker's log holds exactly
   `gap` then `release` (oracle: 035's own stop; the stopping mark ends the steering's
   wait).
10. **Given** the rig, the setting and the scripted tracker of US7-S9, **When** a test
    starts a stack, calls the simulator's `set_unresponsive(true)` once `stack.taken` is
    1, while the second gap waits, and waits until `stack.state` is `ended`, **Then** a
    committed test asserts that `stack.ended_reason` is `camera`, `stack.taken` 1 and
    `stack.error` exactly `the scope node is not connected`, the take's own refusal: the
    exposure is set once, before the first sub; and that, after
    `set_unresponsive(false)` and a wait until the imaging view's `state` is `ready`,
    `settings.exposure_ms` is 30000.0: the camera refused the stack's change back to the
    observer's 60 s, and a refusal there changes nothing else (oracle: FR-023, and 031's
    own words for a camera that is not ready).

---

### User Story 8 - The readout on the Imaging page and in the session (Priority: P3)

As the observer, I see on the Imaging page what steers the tracker, how large the
tracking error is, how far the target drifts during a sub, in arcseconds and in pixels,
whether my exposure was cut, and whether the stack is waiting for the tracker. I set the
corner smear, the switch and the two tuning shares there.

**Why this priority**: It is how the observer knows the feedback works, and what the
field check reads. It is P3 because the feedback works without it.

**Independent Test**: `crates/finder/tests/console/steering_readout.rs`, the module
`steering_readout` of `console`, in the `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` (during
development, `cargo nextest run -p finder --locked --test console steering_readout::`);
and `npm test` in `crates/finder/web`, which runs
`src/components/SteeringBlock.test.tsx`.

**A set clock** is a `Clock` of the test module's own whose time the test sets. **A
solve for the tracking error (e, n)** is `ImagingSolve::from_cd` with the worked
solve's matrix and size and the centre RA 83.81867° + e/(3600 × cos 5.38967°), Dec
−5.38967° + n/3600, so that with the worked target and the centre as set point the
tracking error is (e, n) within 10⁻⁴″.

**Acceptance Scenarios**:

1. **Given** a scripted steering on a set clock, whose tracker's target is the worked
   target and answers `Stepped` (0, 0) at its first two gaps and (−0.5, 0.0) at its
   third, **When** a test, for the times T = 1768528800.0, T + 6.5 and T + 13.0 in
   turn, sets the clock to the time, awaits `before_exposure` with the worked ask, and
   calls `accepted` with the worked frame (`start` that time, `exposure_s` 5.0) and a
   solve for the tracking
   error (0.0, 0.0), (0.6, 0.8) and (0.1, 1.2), **Then** a committed test asserts that
   after the second sub `drift_arcsec` is 0.76923 ± 10⁻⁴ and `error_rms_arcsec` 0.70711
   ± 10⁻⁴ (1/√2, written `std::f64::consts::FRAC_1_SQRT_2` in the test: clippy's
   `approx_constant` refuses the literal); and after the third `drift_arcsec` is 0.58583
   ± 10⁻⁴, `drift_px` 0.27122 ± 10⁻⁴, `error_rms_arcsec` 0.90370 ± 10⁻⁴ and
   `error_rms_px` 0.41838 ± 10⁻⁴ (worked example: the drifts are 1.0″ × 5/6.5 and 0.4″ ×
   5/6.5, the second after the step of (−0.5, 0.0) is taken out; the readout is their
   root mean square, and the errors'; the scale is 2.16″ a pixel). After one sub, none
   of the four keys is present. **And** on another such steering, with the second sub's
   solve at half the worked scale, after the second sub `error_rms_px` is
   `error_rms_arcsec` / 1.08 within 10⁻⁴; and on another, with the first gap answering
   `Stepped` (0.3, 0.0), before the first error, the second sub's `drift_arcsec` is
   still 0.76923 ± 10⁻⁴.
2. **Given** the same steering after 12 subs 6.5 s apart whose errors are (1.0, 0.0) for
   the first two, (2.0, 0.0) for the third and (0.0, 0.0) for the last nine, **Then** a
   committed test asserts `error_rms_arcsec` is 0.63246 ± 10⁻⁴ (√(4/10): the window is
   the last ten, of which the third error is the oldest; a window of nine would give 0.0
   and one of eleven 0.67420). **And** after `release` and a new `before_exposure`, none
   of the four keys is present.
3. **Given** `SteeringBlock` rendered with a view whose `steering` is
   `{"smear_px":6,"on":true,"step_share":0.5,"rate_share":0.3,"source":"subs","holding":true,"steady":true,"waiting":false,"subs":12,"dropped_not_steady":0,"exposure_s":5,"limited":false,"over_limit":false,"longest_exposure_s":9.29,"would_take_s":5,"error_rms_arcsec":0.9037,"error_rms_px":0.41838,"drift_arcsec":0.58583,"drift_px":0.27122}`,
   **Then** a committed test asserts one region named `Tracking from the subs` whose
   status reads exactly `Steered by the subs · 12 used`, and terms reading
   `Tracking error from the subs (RMS, last 10)` with `0.9″ · 0.4 px`, `Drift per sub`
   with `0.6″ · 0.3 px`, `Exposure` with `5.0 s`, `Longest exposure (6 px corner smear)`
   with `9.3 s` and `A stack started now would take` with `5.0 s`; that the readout's
   terms are exactly those five; and no alert.
4. **Given** the same with `limited` true, `exposure_s` 30 and `longest_exposure_s`
   37.2, **Then** a committed test asserts that `Exposure` reads exactly
   `30.0 s, shortened to keep corner stars round` and
   `Longest exposure (6 px corner smear)` reads exactly `37.2 s` (37.2 and not the
   worked limit of 37.15, which a binary float holds a hair under and JavaScript's
   `toFixed(1)` prints as `37.1`: the test is of the words, not of rounding a tie); that
   with `smear_px` 1.5 that term is named `Longest exposure (1.5 px corner smear)`; that
   with `longest_exposure_s` `"no limit"` it reads `none`; that with `over_limit` true
   and `longest_exposure_s` 25.1 an alert reads exactly
   `Corner stars are trailing: the longest exposure for round stars is now 25.1 s, and this stack takes 30.0 s. It keeps running. To shorten the exposure, stop it and start a new stack.`;
   that without `would_take_s` there is no term `A stack started now would take`, and
   with it 15 and no `exposure_s` it reads `15.0 s` and there is no term `Exposure`;
   that with `error_rms_px` absent there is no term
   `Tracking error from the subs (RMS, last 10)`; that with `dropped_not_steady` 2 no
   term names it, 035's own `Dropped` line being where it is shown; and that with no
   `steering` in the view the region shows the form and no status: `Corner smear (px)`
   reads `6`, the box is ticked, `Step share` reads `0.5` and `Rate share` reads `0.3`.
5. **Given** `SteeringBlock` with the view of US8-S3, **When** a test sets the field
   `Corner smear (px)` to 1.5, unticks `Steer the tracker from the subs` and clicks
   `Save tracking settings`, with `fetch` answering 200 and a view, **Then** a committed
   test asserts `fetch` was called with `/api/imaging/steering` and
   `{"method":"PUT","headers":{"Content-Type":"application/json"},"body":"{\"on\":false,\"smear_px\":1.5,\"step_share\":0.5,\"rate_share\":0.3}"}`,
   then `onChange`; and that a 400 answer with
   `{"error":"the corner smear must be a number from 0.1 to 20"}` shows an alert with
   exactly that text and calls no `onChange`.
6. **Given** `ImagingPage` rendered as 031's own test renders it, with a view that holds
   the `steering` of US8-S3, **Then** a committed test asserts the region
   `Tracking from the subs` is present and comes after 035's stack block in document
   order.
7. **Given** `SteeringBlock` with the view of US8-S3, **Then** a committed test asserts
   that the status reads exactly: with `source` `tracker` and `holding` false,
   `Steered by the tracker's own corrections`; with `source` `tracker`, `holding` true
   and `on` true,
   `No sub accepted lately: the tracker's own corrections steer until one is. See Dropped in the stack.`;
   with `source` `tracker`, `holding` true and `on` false,
   `The switch is off: the tracker's own corrections steer`; with `source` `tracker` and
   `far_arcsec` 476.4,
   `The target is 7.9′ from the set point: the tracker steers on its own. Centre it, or turn Track off and on to apply the measured aim point.`;
   and with `source` `none`, `Not tracking: nothing steers`, with no alert while
   `holding` is false and, while it is true, an alert that reads exactly
   `Nothing is tracking, and the stack is still exposing. Turn Track on or stop the stack.`
8. **Given** `SteeringBlock` with the view of US8-S3 and `waiting` true, **Then** a
   committed test asserts that the status reads exactly: with `pause` `starting`,
   `Exposures paused: tracking is starting`; with `reversing`,
   `Exposures paused for a moment: the mount is reversing`; with `measuring`,
   `Exposures paused: the mount is measuring its gear play, once, for a few minutes`;
   with `overhead`,
   `Exposures paused: the target is too near overhead to follow. They resume when it has passed`;
   with `pushed`, `Exposures paused: the telescope is being moved by hand`; with
   `stalled`,
   `Exposures paused for a moment: the mount stopped and is being started again`;
   with `limit` and `exposure_s` 30,
   `Exposures paused: corner stars would trail. They resume when the longest exposure for round stars reaches 15.0 s`;
   and with no `pause`, or with one the page does not know, `flipping`,
   `Exposures paused: the tracker is not steady`. With `waiting` false and `steady`
   false the status is the one its `source` gives, `Steered by the subs · 12 used`: only
   a wait is a pause.
9. **Given** `SteeringBlock` with the view of US8-S3, **When** a test sets the field
   `Corner smear (px)` to 1.5 and renders the block again with the same view, **Then** a
   committed test asserts that the field still reads `1.5`; and that when it is then
   rendered with a view whose `smear_px` is 3, the field reads `3`: the fields are
   drafts of the block's own, which the page's one-second read of the same settings
   leaves alone and a change of the settings replaces.
10. **Given** 035's `StackBlock` rendered with a view whose `stack` is stacking with
    `dropped` holding `not_steady` 2 and every other count 0, **Then** a committed test
    asserts that its line reads exactly `Dropped 2: 2 while the mount was not steady`;
    and that with no `not_steady` key in `dropped` and `behind` 4 it reads
    `Dropped 4: 4 behind`, as before.

---

### User Story 9 - The claim's edges, tested on the tracker (Priority: P1)

As the factory that builds User Story 3, I test in a story of their own what that
story's tracker does at the edges of a claim: an error over the deadband and one against
an axis's motion, the lapse and what it holds, a release, a gap before the first rates,
a followed push, and the finder camera beside all of it.

This story is built after User Story 3 and before User Story 4 (Work Graph). It has the
number 9 because it was cut from User Story 3 on 2026-11-19 (the owner's choice 14 of
the reanalysis): with round 1's cases and the reanalysis's folded in, that story's
estimate passed 60,000 B of the 64 KiB budget, and numbering it so left every other
story with its number. Its thirteen scenarios were US3-S8, US3-S12 and US3-S13 whole,
and the later halves of US3-S2, US3-S3, US3-S5, US3-S6, US3-S7 and US3-S11, which each
held more than one Given, When and Then (§ Clarifications, the session of the
reanalysis, has the table). What they test is User Story 3's code: this story writes
nothing under `crates/finder/src`, and on User Story 3's code its scenarios already pass
(tasks.md, US9's test tasks). Its module writes the tracker rig out again, as every
module that uses it does.

**Why this priority**: A claim that outlived its stack, a lapse that changed a rate
inside an exposure or a followed push that steered back to the old place would each
smear subs with no error shown. It follows User Story 3, whose code it tests, and comes
before User Story 4, since both add a `mod` line to
`crates/finder/tests/motion/main.rs`.

**Independent Test**: `crates/finder/tests/motion/steering_claim.rs`, the module
`steering_claim` of finder's `motion` test binary. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test motion steering_claim::`.

**Acceptance Scenarios**:

1. **Given** the tracker rig with a rate error of 0.01 and the second task of US3-S2,
   which calls `tracker.pointing` every 2 s with the truth error plus Gaussian noise of
   σ 15″ on each of east and north, **When** a test plays 40 subs of 5 s with feedback,
   **Then** a committed test asserts that every true drift from the 12th on is at most
   0.15″: the finder camera's noise steers nothing while the subs steer (oracle: the
   simulator's truth; the noise is the research note's 0.3′ to 0.5′ a finder solve, and
   US3-S2 shows what the same pointings do to subs played without feedback).
2. **Given** the tracker rig with no rate error, **When** a test awaits one `gap`, hands
   the axis displacement (0.0, 2.1 counts) as a tracking error at the time now, and
   awaits a second `gap`, **Then** a committed test asserts that the second gap's
   `Stepped` is within 0.05″ of −0.5 times the error handed over, that the truth error
   changes across the gap by that `Stepped` within 0.05″, that the gap takes between
   0.5 s and 0.6 s, and that every azimuth rate the rate log holds from the gap's call
   to its answer equals the azimuth feed-forward rate at its time within 0.001″/s.
   **And** on another tracker rig, for the axis displacement (3.0 counts, 0.0), the same
   holds with the two axes exchanged: the altitude rates are the ones that stay at the
   altitude feed-forward rate (oracle: the simulator's truth and its transforms; the
   deadband is the larger of two counts, 1.6″ here, and 1″ on the sky, which at M42's
   43° of altitude is 1.37″ of azimuth and 1″ of altitude: two counts on both axes, so
   no figure here moved when the 1″ became an angle on the sky. US3-S3 has an error
   under it).
3. **Given** the tracker rig with no rate error and a first gap, **When** the axis
   displacement (60.0″, 0.0) is handed as an error at the time now and a second gap
   awaited, **Then** a committed test asserts that the `Stepped`'s length is within 0.3″
   of the length of the axis displacement (r/2, 0.0), r being the azimuth feed-forward
   rate then in arcseconds a second, about 19.9: a step against the axis's motion can
   only hold the axis still for the half second (oracle: the simulator's truth and
   transforms; the azimuth axis turns forward at M42's place then, and the step this
   error asks for runs back against it. US3-S5 has the steps that run with the axis).
4. **Given** the tracker rig with no rate error and a first gap, **When** a test awaits
   eight more gaps with no `steer` between them, the eighth called 0.2 s after the
   seventh answers, and then a ninth, **Then** a committed test asserts that
   `steered_by_subs()` is true after the seventh and false after the eighth, that the
   rate log gains no rate from the eighth gap's call to its answer, and that the ninth
   leaves it false and answers exactly (0.0, 0.0): a claim that lapsed stays lapsed
   while no tracking error comes. **When** the test then calls
   `tracker.pointing(30.0, 0.0)` twice 2 s apart, **Then** neither changes a rate: every
   rate the log gains from the first call until the next gap's call equals its axis's
   feed-forward rate within 0.001″/s, and the tracking offset stays exactly `[0.0, 0.0]`
   (US3-S11 says what a gap does with those pointings). **When** it then calls
   `tracker.steer` with (0.0, 0.0) at the time now and awaits one more gap, **Then**
   `steered_by_subs()` is true again, that gap answers exactly (0.0, 0.0), and the
   tracking offset read after it is the one read just before it, exactly: the error that
   takes the tracker back sets no rate, and the step held from the two pointings is
   forgotten. **When** it awaits eight more gaps with no `steer` and one after them,
   **Then** `steered_by_subs()` is false again and that last gap answers exactly (0.0,
   0.0): nothing held before the claim was taken back survives it (oracle: the rig's
   rate log and the test's own feed-forward rate; the eight are 035's queue of four
   waiting subs, the one in work and the three of before).
5. **Given** the tracker rig with no rate error and a first gap, **When** a test calls
   `release()`, **Then** a committed test asserts that `steered_by_subs()` is false at
   once; that a `steer` of (12.0, −16.0) at the time now, made after it, is dropped, so
   the next gap answers exactly (0.0, 0.0); and that this gap makes `steered_by_subs()`
   true again, as a new stack's first gap does. **And** on another rig, after a first
   gap, a `steer` of (12.0, −16.0) at the time now and then `release()` leave the next
   gap answering exactly (0.0, 0.0): `release` forgets an error not yet applied. **And**
   on another rig whose claim has lapsed as in US9-S4, `release()` and one more gap make
   `steered_by_subs()` true: a released tracker is claimed by the next gap, lapsed or
   not (oracle: FR-011).
6. **Given** the tracker rig with no rate error and a first gap, **When** a test awaits
   eight more gaps, each preceded by a `steer` at the time now whose east is NaN,
   **Then** a committed test asserts that `steered_by_subs()` is false after the eighth:
   an error that is not finite is ignored and keeps no claim. **And** on another rig a
   `steer` of (12.0, −16.0) at the time now, made before any gap, leaves the next gap
   answering exactly (0.0, 0.0) with a tracking offset of exactly `[0.0, 0.0]`: an error
   handed over before a stack's first gap is dropped (oracle: FR-011).
7. **Given** a tracker rig with no rate error, built without the wait for its first
   rates, **When** a test awaits `gap` as the first thing after `start` answers,
   **Then** a committed test asserts that it answers exactly (0.0, 0.0), that
   `steered_by_subs()` is false after it, and that, 0.1 s later, the first rate the log
   holds for each axis equals that axis's feed-forward rate at its time within 0.001″/s:
   a gap that comes before a tracker's first rates claims nothing and sends nothing, so
   no axis is sent a zero before it has started (oracle: the rig's rate log and the
   test's own feed-forward rate).
8. **Given** the tracker rig with no rate error, **When** a test drops a `gap` future
   after one poll (a timeout of zero) and waits 0.1 s, **Then** a committed test asserts
   that `steered_by_subs()` is false: a gap nobody waits for claims nothing (oracle:
   plan.md § Plan traps 18).
9. **Given** the tracking console, **When** a test waits 4.5 s after the session first
   reports `tracking`, so that the tracker has sent its first rates and, from US6,
   settled, takes the session's tracker (`state.session().tracker()`), awaits `gap`, and
   for 20 s hands the error (0.0, 0.0) at the time now and awaits `gap` every 2 s,
   **Then** a committed test asserts that the newest sync's `time` in the session's
   pointing model advanced by at least 15 s over those 20 s (the model keeps one sync of
   a place and replaces it, `crates/finder/src/model.rs:512`), that the tracker's
   `state().samples` grew, that `steered_by_subs()` is true throughout, and that the
   tracking offset at the end is the one read after the first gap within 0.002″/s
   (oracle: the session's own model; the finder camera still feeds it).
10. **Given** the tracker rig with no rate error, a first `gap` and eight more gaps,
    each after 6 s of waiting and with no `steer` between them, so that the claim has
    lapsed, **When** a test calls `tracker.pointing(100.0, 0.0)`, waits 2 s, calls it
    again, waits 1 s and awaits a gap, **Then** a committed test asserts that the gap's
    `Stepped` has a length within 3% of that of the axis displacement (30.0″/cos a, −0.6
    × d₁), a being the altitude of M42's apparent place then and d₁ the altitude part of
    a sky offset of (100.0, 0.0) by the differencing of
    `crates/finder/tests/motion/tracking/pi_loop.rs:481`: a held step moves an axis 30″
    on the sky at most, about 33″ in all here, against 60″ unclamped (oracle: the
    simulator's truth and its horizon transform, differentiated by the test, and 015's
    constants. US3-S11 has a held step under the limit).
11. **Given** the tracker rig with a rate error of 0.01 and the second task of US3-S2,
    which calls `tracker.pointing` every 2 s with the truth error plus Gaussian noise of
    σ 15″ on each of east and north, **When** a test awaits a first gap, eight more 6 s
    apart with no `steer`, and then ten more 6 s apart, **Then** a committed test
    asserts that `steered_by_subs()` is false after the ninth gap and after every later
    one; that from the answer of each of the ten last gaps to the next gap's call, and
    after the last of them for the 6 s that follow its answer, every rate in the log
    equals its axis's feed-forward rate plus the tracking offset read just after that
    answer within 0.001″/s; that the tracking offset read after the last of the ten is
    exactly the one read after the ninth gap; and that at least one of the ten gaps
    answered a `Stepped` longer than 1.0″: once the subs have lapsed, the finder
    camera's corrections reach the mount in the gaps, and only there, and only as steps
    (oracle: the rig's rate log and the test's own feed-forward rate; with σ 15″ on each
    coordinate a pointing falls inside 015's 10″ deadband at most about one time in
    five, 1 − e^(−100/450), so the chance that the newest pointing before each of those
    ten gaps was inside it every time is under 10⁻⁶, whatever the seed).
12. **Given** the tracker rig with no rate error, **Then** a committed test asserts that
    after a first gap, 6 s waited and a `steer` of (12.0, −16.0) at the time now,
    `follow` to a place 10′ east (of M42's catalogue place) answers Ok,
    `steered_by_subs()` is false at once, and the next gap answers exactly (0.0, 0.0)
    and claims again; that a `steer` of (−600.0, 0.0) whose time is 2.5 s before the
    `follow`, handed over after that gap, leaves the gap after it answering exactly
    (0.0, 0.0); and that, 1 s later, a `steer` of (12.0, −16.0) at the time now makes
    the next gap answer within 0.1″ of (−6.0, 8.0) (oracle: 038's `Tracker::follow` and
    FR-011: an error measured from the old place steers nothing, whether it was waiting
    at the `follow`, as `release` forgets one, or is handed over after the claim was
    taken again, as 035's worker hands over a sub taken before the push. −600″ east is
    such a sub's error against the new place; the recheck of round 1 ran that case
    before the rule was written and saw a step of (7.49, 0.02)″).
13. **Given** the tracker rig with no rate error, a first gap and eight more with no
    `steer`, so that the claim has lapsed, and `tracker.pointing(30.0, 0.0)` called
    twice 2 s apart, **When** the test calls `follow` to a place 10′ east of M42's
    catalogue place and awaits a gap, **Then** a committed test asserts that `follow`
    answers Ok, and that the gap answers exactly (0.0, 0.0) and makes
    `steered_by_subs()` true: `follow` forgets the step a lapsed claim held and clears
    the lapse, so the stack's next gap claims (oracle: FR-011; without it that gap would
    step about (−18.0, 0.0), as in US3-S11, and claim nothing).

---

### User Story 10 - Not steady at its edges, tested on the tracker (Priority: P2)

As the factory that builds User Story 6, I test in a story of their own what that
story's tracker does around a time it is not steady: what the mount is left running at
the first gap after one, a first reversal's measurement and its close, a start and a
stop, what a take-up forgets, the keyhole pass, and subs steered through a reversal.

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

**Why this priority**: The first exposure after a take-up, a measurement or the keyhole
pass is the one most likely to start on a rate nobody meant, and a stop that left a gap
unanswered would hang a stack for good. It follows User Story 6, whose tracker it tests,
and comes before User Story 7, since both add a `mod` line to
`crates/finder/tests/motion/main.rs`.

**Independent Test**: `crates/finder/tests/motion/steering_unsteady.rs`, the module
`steering_unsteady` of finder's `motion` test binary, on the backlash rig. The `test`
gate's `cargo nextest run --workspace --all-targets --all-features --locked` runs it;
during development,
`cargo nextest run -p finder --locked --test motion steering_unsteady::`.

**Acceptance Scenarios**:

1. **Given** the lapsed backlash rig and a `pointing(0.0, 0.0)` right after its ninth
   gap, **When** a `pointing(30.0, 0.0)` arrives once the take-up at transit has made
   `steady()` false and a `gap` is awaited as soon as `steady()` reads true again, both
   read every 10 ms, **Then** a committed test asserts that the azimuth rate in force
   when the gap answers, the last in the rate log at or before its answer, is the
   feed-forward rate (as `crates/finder/tests/motion/tracking/pi_loop.rs:252` computes
   it) plus the tracking offset read just after the answer, within 0.01″/s, and that so
   is the first rate after it (oracle: the rig's rate log. The first pointing gives the
   second an interval of about 50 s, so the proportional rate 015 sets for the second,
   about +0.5″/s of azimuth, outlasts the settling; with no earlier pointing it would
   last 1 s, and a turn would have dropped it before the tracker was steady again.
   US6-S8 and US10-S9 are the two ways the tracker's own state stops showing that rate
   while the mount still runs it).
2. **Given** `crate::support::tracker::Rig` for M42 from 120 s before its transit on a
   fresh session, so that no backlash is measured, followed to transit + 30 s, while the
   first reversal is measured, **Then** a committed test asserts that `steady()` is
   false, that `pause()` is `Some("measuring")`, and that a `gap` awaited then answers
   exactly (0.0, 0.0) and leaves `steered_by_subs()` false. **When** the rig is followed
   on to transit + 600 s with a reader that, every 50 ms, notes whether
   `GET /api/session`'s `backlash.alt_deg` is present and then `steady()`, with no await
   between the two, **Then** some reading has the backlash measured, and every reading
   that has it has `steady()` true (oracle: 015's own measurement, which the session
   reports once it has closed).
3. **Given** the backlash rig as `Rig::new` leaves it, with the console time s read as
   soon as it answers and no wait, **Then** a committed test asserts that the rate log
   holds no rate, `steady()` is false and `pause()` is `Some("starting")`; that a `gap`
   awaited then answers exactly (0.0, 0.0) and leaves `steered_by_subs()` false; that
   0.1 s later the log holds a rate for each axis and `steady()` is still false; that it
   is false 1.9 s after the first rate in the log and true 3.1 s after it, with
   `pause()` then `None`: the tracker settles for 2 s after its first rates, to the end
   of one of its once-a-second turns; that `steady_between(s - 20.0, s - 10.0)` is
   false: it was not tracking then; that `steady_between(now - 10.0, now)`, now being
   the console time when it is asked, is false asked 6 s after s and true asked 14 s
   after s: the window 037's mount part asks, ending now and reaching back the larger of
   10 s and the exposure; and that after `rig.follow(s + 20.0, 60.0)` and
   `stop("stopped")`, with e the console time read just before the stop, `steady()` is
   false, `steady_between(e - 3.0, e + 0.1)` and `steady_between(e + 1.0, e + 2.0)` are
   false, and `steady_between(e - 3.0, e - 1.0)` is still true (oracle: the rig's rate
   log and clock).
4. **Given** the lapsed backlash rig and, 30 s after its ninth gap, a
   `pointing(30.0, 0.0)` (held, since the tracker is steady and lapsed), **When** the
   take-up at transit has made `steady()` false and then true again, both read every
   10 ms, and a `gap` is awaited then, **Then** a committed test asserts that it answers
   exactly (0.0, 0.0): what was held when the tracker stopped being steady was forgotten
   (awaited during the take-up, the gap would be turned away while the tracker settles,
   and prove nothing).
5. **Given** the backlash rig followed to 60 s before the transit with
   `rig.follow(transit - 60.0, 60.0)`, one `gap` awaited then and, 50 s later, a `steer`
   of (12.0, −16.0) at the time then, **When** the take-up has made `steady()` false and
   then true again, both read every 10 ms, and a `gap` is awaited then, **Then** a
   committed test asserts that it answers exactly (0.0, 0.0) and that
   `steered_by_subs()` is true: the take-up dropped the claim and the error not yet
   applied, and that gap took the claim back.
6. **Given** the backlash rig at transit − 120 s with the console time s read at once,
   and `let steered: &dyn Steered = &rig.tracker`, **Then** a committed test asserts
   that `steered.steady()` is false and `steered.pause()` is `Some("starting")`; and
   that after `rig.follow(transit + 10.0, 60.0)`, which crosses the take-up,
   `steered.steady()` is true, `steered.pause()` is `None`,
   `steered.steady_between(s - 20.0, s - 10.0)` is still false,
   `steady_between(transit - 5.0, transit + 5.0)` is false and
   `steady_between(s + 5.0, s + 20.0)` is true: the trait answers as the tracker does,
   still, after a take-up.
7. **Given** the backlash session and a clone of M42 at Dec +39°, as
   `crates/finder/tests/motion/tracking/keyhole.rs` uses, from 300 s before the first
   whole minute at which its altitude reaches 88°, with the rig's true error given as a
   pointing every 2 s, **When** the test reads `state().state`, `steady()` and `pause()`
   every 50 ms, for at most 1800 s of the rig's clock, until the tracker is `tracking`
   and steady after a `keyhole_wait`, **Then** a committed test asserts that a
   `keyhole_wait` was seen, that every reading in it has `steady()` false and `pause()`
   `Some("overhead")`, that the tracker is steady at the end, and that `steady_between`
   over the 2 s either side of the first `keyhole_wait` reading, asked at the end, is
   false (oracle: 015's keyhole pass, as its own test drives it).
8. **Given** the backlash rig with `rig.mount.set_rate_error(0.01)` called as soon as
   `Rig::new` answers, **When** a test plays subs of 5 s late until the rig's clock
   passes transit + 120 s, with three changes to playing a sub: the truth error is
   `rig.truth_error()`; before each sub it waits, reading every 50 ms, until `steady()`
   is true; and a sub for which `steady_between`, asked as the sub ends for the console
   times of its e0 and e1, is false is left out, its drift not judged and its error not
   handed over, **Then** a committed test asserts that `steady()`, read at each of the
   player's 50 ms steps, read false at least once; and that, of the subs whose e0 was
   read after it next read true, every one from the fourth on has a true drift of at
   most the bound probe round 2 measures (oracle: the simulator's truth. This scenario's
   bound is not written yet: no build has steered through a reversal with a rate error,
   and the number is to be put here, with its margin, before this spec is flipped; a
   drift under 0.15″ is expected).
9. **Given** the lapsed backlash rig and a `pointing(0.0, 0.0)` 5.4 s before the
   transit, **When** a `pointing(30.0, 0.0)` arrives once the take-up at transit has
   made `steady()` false, at the console time p, and a `gap` is awaited at the first
   10 ms step at which `steady()` reads true and the console time is at least p + (p −
   (transit − 5.4)) + 0.01, **Then** a committed test asserts that the last azimuth rate
   the log holds before that gap's call differs from the feed-forward rate plus the
   tracking offset by more than 1″/s, and that the azimuth rate in force when the gap
   answers, the last in the rate log at or before its answer, is the feed-forward rate
   plus the tracking offset read just after the answer, within 0.01″/s (oracle: the
   rig's rate log. The second pointing's proportional rate lasts for its interval, p −
   (transit − 5.4), about 3.4 s: the turn that ends the settling still sends it, and its
   time runs out before the gap, so the tracker's own state says no rate is in force
   while the mount runs one until the next turn. The first assertion shows that the case
   was met. The 5.4 s is worked, not measured: from the take-up's start as the recheck
   of round 1 saw it, 1.99 s before the rig's transit, and the 3.0 s from there to the
   turn that ends the settling; probe round 2 confirms it or moves it).

---

### User Story 11 - An MCP client sets the steering (Priority: P3)

As an agent that runs a night through the console's MCP tools, I set what the page's
form sets: whether the subs steer, the corner smear, and the two shares. I read the
steering where I already read the camera, in `get_imaging_camera` and the session tools.

This story is the alternative of owner decision 11, taken by the owner on 2026-11-19
(the reanalysis, choice 12). It is built last (Work Graph), after the page's story,
because it edits the README's list of tools after that story's section.

**Why this priority**: 037 gives an agent the whole night, and without the tool an agent
could neither compare a stack with and without the feedback nor change the smear before
a restart. It is P3 because the page does the same for an observer.

**Independent Test**: `crates/finder/tests/mcp/steering.rs`, the module `steering` of
finder's `mcp` test binary, which goes through the MCP door. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test mcp steering::`.

**The imaging console** of this story is a console that takes imaging frames, built in
the module on a stand-in scope node whose camera is ready, as 031's own MCP tests build
theirs (031 spec.md US8), and reached through the MCP door as they reach it.

**Acceptance Scenarios**:

1. **Given** the imaging console, **Then** a committed test asserts that `tools/list`
   names `set_steering` with the description
   `Set what steers the tracker during a live stack, as PUT /api/imaging/steering does: on, smear_px, step_share and rate_share, each optional. Read the result in the steering part of get_imaging_camera.`,
   an `outputSchema` of `{"type":"object"}`, no `readOnlyHint`, and an `inputSchema`
   whose `properties` are exactly `on`, `rate_share`, `smear_px` and `step_share` and
   which requires none of them.
2. **Given** the imaging console, **Then** a committed test asserts that `set_steering`
   with `{"smear_px":1.5}` answers the imaging view, whose `steering.smear_px` is 1.5
   and `steering.on` true; that with `{"on":false,"step_share":0.25,"rate_share":0}` it
   answers `steering.on` false, `steering.step_share` 0.25 and `steering.rate_share`
   0.0, with `smear_px` still 1.5; that `get_imaging_camera` then carries the same
   `steering`, and `get_session`'s `imaging.steering` too; and that `{}` answers the
   view and changes nothing (oracle: the route's own answers, US5-S5).
3. **Given** the imaging console, **Then** a committed test asserts that `set_steering`
   with `{"smear_px":0}` answers a tool error whose text is exactly
   `the corner smear must be a number from 0.1 to 20`, with `{"step_share":1.5}` exactly
   `the step share must be a number from 0 to 1`, and with `{"smear":2}` and with
   `{"on":1}` a tool error, and that none of the four changes anything; and that on a
   console that takes no imaging frames `set_steering` with `{}` answers the tool error
   `this console takes no imaging frames`, with no tokio time passed (oracle: the
   route's own refusals; an existing test calls every listed tool with `{}` on such a
   console, and a tool that waited there would hang it).

---

### Edge Cases

- **No Track.** With Track off, or armed and waiting for a GoTo, there is no tracker to
  steer: the stack is neither capped nor paused and hands over nothing (US7-S4). Subs
  taken during a slew fail 035's own gates.
- **Tracking ends during a stack.** The sub in flight is dropped as `not_steady`; later
  subs find no tracker. Stopping the stack is 035's and 037's; until then the block says
  that nothing is tracking and the stack is still exposing (US8-S7).
- **Stop.** Stop ends tracking as before. A gap that is waiting answers at once with no
  step, one still queued behind a take-up included, and no rate follows the Stop
  (US3-S7, US6-S10).
- **A push by hand, not yet settled.** From the live pointing that makes 038's push
  `moving` until the push is followed or the telescope is back within 038's threshold,
  the tracker is not steady: exposures pause, the sub in flight is dropped as
  `not_steady`, and the claim and any error not yet applied are dropped as for a
  take-up. When the push is followed or comes back, the stack's next gap claims again.
  One finder solve more than 5′ wrong costs one sub in the same way (US6-S6).
- **A followed push.** 038's `follow`, as `release` does, ends the claim, clears a lapse
  and forgets what was pending or held: an error measured from the old place steers
  nothing, whenever it is handed over, and the stack's next gap claims again (US9-S12,
  US9-S13).
- **Subs that are not accepted.** Each is a gap with no tracking error. After eight gaps
  in a row with none, the tracker's own corrections steer again, with the tracking
  offset as it stood, and go on steering through every later gap until a sub is
  accepted, applied only in the gaps and only as steps: a finder pointing between two
  gaps holds its step for the next and changes no rate (US3-S11). The gap after that
  sub's tracking error takes the tracker back, forgets what was held and steps without
  setting a rate: no drift rate is measured across the break (US9-S4).
- **The switch off.** No stack claims the tracker, so its own corrections run as 015 has
  them, during exposures too: the observer's declared exception (owner decision 8).
  Turned off while a stack runs, it lets the tracker go at once; turned on again, the
  stack's next gap claims it (US5-S9).
- **A set point named during a stack.** A set point that changes lets the tracker go in
  the same way, so that no drift rate is measured across the move, and the stack's next
  gap claims afresh (US4-S12).
- **A gap before a tracker's first rates.** It answers at once, moves nothing and claims
  nothing (US9-S7); from US6 the stack does not ask for one, because such a
  tracker is not steady yet.
- **A tracking error that arrives late.** It is judged against the steps and rate
  changes made since its mid-exposure (US3-S9, US3-S10). Of two that arrive between two
  gaps, the newer is used.
- **The worker falls behind, or is slow.** A gap with no tracking error counts toward
  the lapse whatever the reason: a sub dropped as `behind`, `settings`, `frame_failed`,
  `unreadable` or `solver_failed`, or a sub still being solved. The lapse comes at the
  eighth such gap in a row, so a worker a whole queue behind (035 keeps four subs
  waiting and one in work) does not lapse the claim. A console that needs more than
  eight sub lengths for one sub lapses between every two tracking errors: each error
  then steps the telescope and takes no drift out of the rate.
- **A wrong imaging solve, a bump.** 035 does not accept a sub whose solve differs from
  the reference sub's in scale or lies off the field, so it steers nothing. For one that
  passes, and for a tube that was knocked between two subs, one gap moves an axis 30″ on
  the sky at most, a jump between two subs faster than twice the largest tracking offset
  changes no rate, and no gap changes an axis's rate by more than 0.1″/s (US3-S5,
  US3-S8).
- **A mount that does not make the step asked for.** A pair of errors between which the
  gaps stepped an axis by more than two deadbands changes no rate on that axis. So while
  a large offset is taken out, a pulse that moves the tube a little more or less than
  asked is not read as a drift; the rate is measured once the steps are small (US3-S4).
- **A stack that starts far from its set point.** When a stack's first tracking error is
  over 180″ the steering stands back for the rest of that stack: it lets the tracker go
  and calls neither `gap` nor `steer`, the tracker's own corrections run as 015 has
  them, the cap, the pause and the drop still apply, and the view says how far out the
  target is. The finder camera holds the target on its aim point and the subs on the set
  point; steering between two that differ by more than 038's 5′ would read as a push by
  hand (owner decision 7; US4-S8, US7-S7).
- **A rate command that fails at the scope node.** The node stops both axes and still
  acknowledges the command. The tracker's next turn finds the azimuth axis at rest
  although the rate it last sent was not zero and reads not steady from that rate to its
  own next rates, which start the axes again: the sub in flight is dropped as
  `not_steady` and steers nothing (US6-S7). What the node does with a failed command is
  another spec's.
- **Settling.** For 2 s after the rates that follow a tracker's start, a take-up and the
  keyhole pass the tracker is not steady, and no exposure starts: a real tube rings
  after each (US6-S1, US10-S3).
- **The first reversal of an axis on this mount.** Its backlash is still to be measured
  and the measurement takes minutes (altitude reverses at transit, and azimuth too for a
  target that culminates north of the zenith); the tracker is not steady for all of it,
  exposures pause, and the finder camera steers as 015 has it, even when the subs had
  lapsed before it (owner decision 17, US6-S5).
- **A correction against an axis's motion.** An axis is never reversed by a step or a
  rate: a step against its motion can at most hold the axis still for the half second,
  and what is left waits for the next gap (FR-008; US9-S3). That is the axis's own rate
  times half a second a gap: about 10″ of azimuth and 1.8″ of altitude on the tracker
  rig, and nearly nothing where an axis crawls near its reversal. A target left 120″ too
  high east of the meridian is so brought in over about 67 gaps, not four.
- **A long gap.** The feed-forward rates still go out once a second. Nothing times a gap
  out: a stack that is waiting for a tracker in the keyhole waits until it leaves, and
  stopping the stack ends the wait. The block says why exposures are paused (US8-S8).
- **A sub of a region, or binned.** The set point is a place on the sensor; it need not
  lie inside the frame (US4-S4).
- **The limit falls under the exposure in use.** The exposure was decided from the
  shortest limit of the hour ahead (US5-S10, US5-S11), so this comes to a stack more
  than an hour old, or to one whose exposure the floor of 1 s kept above that limit. The
  stack keeps its exposure and the readout says `over_limit`; a new stack starts shorter
  (US5-S2). When the limit falls under half the exposure held, the stack takes no
  exposure until the limit has risen again, lets the tracker go meanwhile, and the block
  says so (US6-S17, US8-S8).
- **The cap near the zenith.** The limit shrinks toward the keyhole. The exposure is not
  halved to under 1 s, nor past the camera's shortest where that is longer, and may then
  be both `limited` and `over_limit` (US5-S1); with the limit under half of it the stack
  waits.
- **A stack started while the limit is low.** It holds a short exposure for its whole
  life while the limit climbs. The block's `A stack started now would take` says what a
  new stack would hold (US5-S12, US8-S4).
- **The observer edits the exposure during a stack.** The edit is refused: while the
  stack holds the camera's lease, `PUT /api/imaging/settings` answers 409 with the
  stack's name (035 plan.md § The camera's lease; 035 US11-S3). So the exposure a stack
  holds cannot change under it. The observer stops the stack, edits and starts again;
  the setting they then see is their own, put back when the stack ended.
- **The exposure cannot be shortened.** If the camera refuses the stack's change of the
  exposure before its first sub, which it does only when it is no longer ready, the
  stack ends with the reason `camera` and the refusal's words, and no sub is taken at
  the observer's longer exposure (US7-S5).
- **The observer's exposure cannot be put back.** If the camera is no longer ready when
  a stack ends, the change back is refused and the setting stays at the exposure the
  stack used; the stack's own ending is not changed by it (US7-S10). The page shows the
  setting.
- **Master darks.** A halved exposure needs a master dark of its own length; without
  one 034's fallback applies and says so.
- **Count steps.** With the simulator's count steps on, the truth read at an instant
  jitters by up to a count; the scenarios that bound a drift leave them off.
- **`--simulate`.** The simulated solver and camera steer the simulated tracker as the
  real ones would, with no flag.

## Requirements *(mandatory)*

### Functional Requirements

**The simulator** (US1):

- **FR-001**: The simulator SHALL remember the direction of its telescope for the last
  3600 s of its clock, one sample for every read of it, and begin again when the
  telescope is pushed by hand through the simulator (`push_to`, `push_by`,
  `point_eyepiece_at`), a mount is connected or the finder's optics are set
  (`set_optics`); a push made on the mount itself (`SimMount::push_by`,
  `SimMount::push_to`) is a motion like any other and keeps the memory. 030's
  `imaging_truth_at` and the imaging camera's own rotation SHALL
  take the direction for an instant between two remembered samples by a straight line
  between them, azimuth the short way round, and otherwise the direction now, as before
  (owner approval 5). `directions_at` SHALL not change.
- **FR-002**: The simulated mount SHALL take `set_count_steps`, false when built, under
  which each axis's tube follows its drive rounded down to a whole count. With it false
  nothing SHALL change.

**The tracking error** (US2):

- **FR-003**: `tracking_error` SHALL answer, for an imaging solve, a set point in that
  frame's pixels, a target's J2000 place and a time, the sky at the set point less the
  target: the solve's matrix applied to the set point less the target's pixel, as east
  and north arcseconds, with the time given; and `None` when the target has no pixel in
  that solve or a number is not finite.
- **FR-004**: `set_point_in_frame` SHALL place a set point given in sensor pixels, or
  the sensor's centre when none is given, in a frame's own pixels from its region and
  binning.
- **FR-005**: `crates/finder/src/imaging/steering.rs` SHALL define `Steered`, the one
  interface between the stack and a tracker: the target's place, `steer`, `gap`,
  `release`, `steered_by_subs`, and the two answers, `longest_exposure_s` for a turn of
  the field, with `longest_exposure_ahead_s` for the hour ahead (none by default), and
  `steady` with `steady_between` (true by default); with them `pause`, the tracker's own
  word for why it is not steady (none by default), and `tune`, which takes the two
  shares of FR-030 (nothing by default). The file SHALL hold none of `ephemeris`,
  `alt_az`, `parallactic`, `counts_per_revolution`, `AxisInfo`, `MountClient`,
  `azimuth`, `altitude`, `field_rotation`, `hour_angle`, `meridian`, `backlash` and
  `keyhole`: it computes no altitude, azimuth, parallactic angle or field rotation and
  reads no axis count, as the plan document's rule 1 asks of every file under
  `crates/finder/src/imaging/` (FR-023 holds 035's `stack/service.rs` to the same).
- **FR-006**: `GLOSSARY.md` SHALL define **Tracking error** and **Set point** (owner
  approval 4).

**The tracker** (US3):

- **FR-007**: A tracker SHALL be steered by the subs from a stack's first `gap` that
  comes while it is tracking and after its first rates (from US6: while it is steady,
  FR-021). While it is, a live pointing from the finder camera SHALL change no rate
  and no tracking offset; it SHALL still be watched for the cable's wind, taken as the
  keyhole's resume pointing, kept among the 60 s of samples and given to the backlash
  measure. The live loop's syncs SHALL not change (owner approvals 1 and 2).
- **FR-008**: At a gap that follows a new tracking error, the tracker SHALL resolve it
  into its two axes as it resolves a finder error, add to each axis's error the steps
  its gaps made after the error's time, and for each axis whose sum is at least the
  larger of two of that axis's counts and 1″ on the sky, move that axis by the step
  share of the sum, −0.5 of it unless tuned (FR-030), at most 30″ on the sky, as one
  rate pulse of 0.5 s added to the axis's rate, never commanding a rate against the
  axis's direction: a step against an axis's motion is so at most that axis's rate times
  0.5 s. It SHALL take the step it made to be the pulse's rate over the time the pulse
  really lasted, from the acknowledgement
  of its start to that of its end.
- **FR-009**: At the same gap, for each axis, the tracker SHALL add to the tracking
  offset the rate share of the drift rate, −0.3 of it unless tuned (FR-030), and at most
  0.1″/s either way: this error less the last
  one applied, less the steps made between their two times, plus what every change of
  the tracking offset made after the earlier time would have moved the axis had it been
  in force from that time to the later one, all over the time between the two errors. It
  SHALL change no offset for the first error after a claim, nor when that drift rate is
  more than twice the offset's limit, nor when the steps made between the two errors'
  times add up to more than two of that axis's deadbands, and SHALL keep the offset
  within 015's limit. It SHALL remember what its last 16 gaps did.
- **FR-010**: At every gap served while the tracker is steered or its claim has lapsed,
  except the gap at which the claim lapses (FR-011), the tracker SHALL send each axis
  its rate and answer only when both are acknowledged; the gap at which the claim lapses
  sends nothing. Between a gap's answer and the next gap, while it stays steered or its
  claim stays lapsed, each once-a-second rate SHALL be the axis's feed-forward rate plus
  the tracking offset as that gap left it, or zero where 015 sends zero in place of a
  rate against the axis's direction; a take-up and the keyhole pass send what 015 has
  them send. Every rate SHALL be sent by the task that sends the once-a-second rates, a
  gap SHALL be served between two of its turns, and none SHALL be sent after tracking
  ends. A gap that comes before a tracker's first rates SHALL answer no step, send
  nothing and claim nothing.
- **FR-011**: The tracker SHALL stop being steered by the subs on `release`, at the
  eighth gap in a row with no new tracking error, and when tracking ends. It SHALL keep
  its tracking offset on a `release` and on such a lapse; the end of tracking and a
  keyhole pass zero it, as 015 has them. After a `release` or the end of tracking the
  finder camera's pointings correct it again as in 015. After such an eighth gap the
  claim has lapsed: the tracker SHALL stay so, whatever gaps follow, until a tracking
  error is handed over, and while it stays so a live pointing SHALL change no rate when
  it arrives and SHALL never change the tracking offset. The step that 015's
  proportional term would make for it, −0.6 of its axis error, SHALL be held instead, a
  newer pointing's step replacing an older one's, and the next gap SHALL apply it as one
  0.5 s pulse of at most 30″ on the sky an axis, added to the axis's rate, never
  commanding a rate against the axis's direction. The gap after a tracking error is
  handed over SHALL take the tracker back, forget what was held and treat that error as
  the first after a claim; `release` and the end of tracking SHALL also forget what was
  held. `follow`, as `release` does, ends the claim, clears a lapse and forgets what was
  pending or held. A tracking error SHALL be ignored when it is not finite, when it is
  handed over before a stack's first gap or after a `release`, when it is no later than
  the last applied, when its time is not later than the last `follow` or, from US6, than
  the last time the tracker stopped being steady, and, from US6, when it is handed over
  while the tracker is not steady; of two handed over between two gaps the later SHALL
  be used.
- **FR-012**: `gap` SHALL answer the step it made, as east and north arcseconds on the
  sky, and zero when it made none.

**The steering** (US4):

- **FR-013**: `Steering::before_exposure` SHALL ask its source for the running tracker
  and, with one, await its `gap` before answering; with none it SHALL answer at once.
  It SHALL answer the exposure to take and whether it is shorter than asked, and SHALL
  remember, for the last 32 exposures, when it answered and which tracker it found,
  without keeping that tracker alive.
- **FR-014**: `Steering::accepted` SHALL, for a sub whose mid-exposure falls in an
  exposure it remembers, whose tracker still exists and has a target, hand that tracker
  the tracking error at the sub's mid-exposure from the set point; and hand nothing
  otherwise. When the first tracking error it works out after a `release`, or the first
  of all, is longer than 180″, it SHALL hand over nothing, release the tracker and,
  until its own next `release`, call neither `gap` nor `steer`: the steering does not
  steer a stack that began far from its set point (owner decision 7). The set point
  SHALL be the sensor's centre unless `set_set_point` has named another, and
  `set_set_point(None)` SHALL put it back there. A `set_set_point` that changes the set
  point SHALL let go a tracker a stack holds, as the switch turned off does (FR-019), so
  that the first tracking error from the new place is a first error.
- **FR-015**: `Steering::release`, and the drop of the `Hold` a stack keeps, SHALL
  release the tracker and forget the exposures remembered.
- **FR-016**: The imaging view, and the session's `imaging` object, SHALL carry
  `steering` from the first use of the steering in a run of the console, and before it
  only as FR-018 has it while a tracker runs: `source` (`subs`, `tracker` or `none`),
  `subs` and `last`, `holding`, true while a stack keeps a `Hold`, and `far_arcsec`, the
  length of that first tracking error, while the steering stands back (FR-014; owner
  approval 3). `Session::steered` SHALL answer the running tracker, and
  `Imaging::steering` the console's steering.

**The exposure cap** (US5):

- **FR-017**: The tracker's `longest_exposure_s` SHALL answer the turn it is asked about
  divided by the field rotation at the target now, and none when that rotation is under
  10⁻⁹ rad/s, in agreement with 015's `longest_exposure`. Its `longest_exposure_ahead_s`
  SHALL answer the shortest of those answers for now and for each whole minute of the
  hour ahead, as 015 samples its forecasts, and none when each of them is none.
- **FR-018**: `before_exposure` SHALL ask the tracker about a turn of the smear setting
  divided by the distance, in delivered pixels, from the set point to the farthest
  corner of the frame to be taken. The first call after a `release`, or the first of
  all, SHALL answer the wanted exposure halved until it is no longer than the tracker's
  answer for the hour ahead (FR-017), or until one more halving would pass 1 s or the
  camera's shortest, whichever is longer, with `limited` true when that is shorter than
  the exposure it was asked for; every later call SHALL answer that same exposure and
  that same `limited`, whatever exposure it is asked for, until the next `release`. The
  view SHALL carry `smear_px` and `on`; `exposure_s` and `limited`, the exposure held
  and whether it was cut, absent and false while none is held; `longest_exposure_s`, a
  number or `no limit`, the tracker's answer for now, asked as the view is made for the
  frame the stack takes or, with no stack, the frame the console would take, and absent
  while no tracker runs; `over_limit`, true while that answer is under the exposure
  held; and `would_take_s`, the exposure a stack started now would hold, wherever a
  tracker runs and the console's camera and settings are known. Where they are, the view
  SHALL be carried while a tracker runs even before the steering's first use (FR-016).
- **FR-019**: `PUT /api/imaging/steering` SHALL accept, under the same-origin rule, a
  JSON object with any of `on` (a boolean), `smear_px` (a number from 0.1 to 20),
  `step_share` and `rate_share` (each a number from 0 to 1), or with none of them, and
  answer 200 with the imaging view; 400 for any other JSON body and for a body that is
  not JSON, with `the corner smear must be a number from 0.1 to 20`,
  `the step share must be a number from 0 to 1` or
  `the rate share must be a number from 0 to 1` for a number out of range; 415 for a
  `Content-Type` other than `application/json`; 403 for a foreign origin; and 409 with
  `this console takes no imaging frames` without imaging. The four settings are
  run-only: 6.0, true, 0.5 and 0.3 at every start. With `on` false the steering SHALL
  call neither `gap` nor `steer`, and SHALL still cap and, from US6, still pause and
  drop. Turning `on` to false SHALL let go at once a tracker a stack holds: the tracker
  is released and the count of tracking errors handed over begins again, while the
  exposure held and the exposures remembered are kept; turned on again, the stack's next
  gap claims it (owner approval 3).
- **FR-020**: `crates/finder/README.md` SHALL document the route in its `## API` table,
  and `crates/finder/src/lib.rs` SHALL list it (owner approval 8).

**Not steady** (US6):

- **FR-021**: The tracker's `steady` SHALL be false from its start until 2 s after its
  first rates are sent; from the start of a take-up or of the keyhole pass until 2 s
  after the rates that follow it are sent, the 2 s ending with the first of the
  tracker's once-a-second turns to end at least 2 s after those rates; while an axis's
  backlash is being measured; while the session reports the telescope moving by hand
  (038), from the live pointing that makes the push `moving` until the push is followed
  or the telescope is back within 038's threshold; from one turn's rates to the next
  turn's, when that next turn finds the azimuth axis at rest although the rate last sent
  it was not zero; and after tracking ends (owner decision 17). `steady_between` SHALL
  be false for a window that touches any such time among the last 64, or that begins
  before the tracker was started, while that time is still among the last 64 (the list's
  first entry); a window that begins at the instant such a time ends does not touch it.
  A tracker that stops being steady SHALL stop being steered by the subs and forget any
  tracking error not yet applied and any correction held for a gap (FR-011 says which
  errors it then ignores); a gap served while the tracker is not steady SHALL answer no
  step and claim nothing, and one asked for during a take-up or the keyhole pass SHALL
  be served once that is over. While the tracker is not steady a live pointing SHALL
  correct it as in 015, even when its claim has lapsed. The tracker's `pause` SHALL
  answer, while it tracks and is not steady, one word for the reason: `pushed` while the
  telescope is moved by hand, else `measuring` while an axis's backlash is measured,
  else `starting`, `reversing` or `overhead` for a start, a take-up and the keyhole
  pass, each with the 2 s that follow it; else `stalled` in a turn that found an axis
  stopped; and none otherwise.
- **FR-022**: `before_exposure` SHALL not answer, and SHALL call no `gap`, while the
  running tracker is not steady, with `steady` false and `waiting` true in the view, and
  `pause` the tracker's word for the reason where it gives one; it SHALL answer only
  after a `gap`, where one is called, at whose end the tracker is steady, and the view
  SHALL then have `steady` true, `waiting` false and no `pause`, as it has before any
  wait. While a `gap` is awaited it SHALL go on reading the tracker, and show `steady`
  false and `waiting` true while the tracker is not steady. It SHALL also not answer,
  with `waiting` true and `pause` `limit`, while the tracker's answer for now (FR-017)
  is under half the exposure held, and SHALL let the tracker go for that wait as the
  switch turned off does (FR-019). `Steering::check` SHALL answer `Drop("not_steady")`
  for a sub when the tracker that `before_exposure` found before that sub's exposure no
  longer exists, or does not say it was steady for the whole of the exposure; and `Keep`
  otherwise, a sub taken with no tracker among them. The view SHALL count one stack's
  drops in `dropped_not_steady`, from 0 again when the next stack takes its `Hold`.

**The stack** (US7):

- **FR-023**: 035's taker SHALL keep a `Hold` while it runs and await `before_exposure`
  before every exposure, a stop of the stack ending the wait. Before its first exposure,
  when the exposure answered differs from the observer's, it SHALL set the console's
  exposure to the one answered through the stack's own lease on the camera; if that is
  refused it SHALL end the stack with the reason `camera` and the error
  `the exposure could not be set to {ms} ms: {the refusal's words}`, having taken
  nothing. The taker SHALL, on every return, put the observer's exposure back through
  the same lease before the lease drops with it, and a refusal there SHALL change
  nothing else. 035's worker SHALL ask `check` for every sub before it solves it and
  count a sub it is told to drop under `not_steady`, a twelfth count of `dropped`; and
  SHALL call `accepted` for every sub it stacks (owner approval 6).
  `crates/finder/src/imaging/stack/service.rs` SHALL hold none of the thirteen names
  FR-005 lists.
- **FR-024**: Giving the console its imaging solver SHALL give its steering the
  session's running tracker as its source, in the binary and under `--simulate` alike.
- **FR-025**: `crates/finder/README.md` SHALL gain a section on tracking from the subs:
  what steers when, the gap, the cap, the pause and its reasons, what each key of the
  `steering` part means, why subs are dropped, and the constants (owner approval 8).

**The readout** (US8):

- **FR-026**: The view SHALL carry, once two tracking errors have been handed over
  since the tracker was last released, `error_rms_arcsec` and `drift_arcsec`, each the
  root mean square over the last ten tracking errors, and the same in delivered pixels
  as `error_rms_px` and `drift_px`, by the latest solve's scale. A sub's drift SHALL be
  the change in tracking error since the sub before, less the steps answered by the
  gaps between their mid-exposures, times that sub's exposure over the time between the
  mid-exposures, worked out when its tracking error is handed over.
- **FR-027**: The Imaging page SHALL show a region `Tracking from the subs` after 035's
  stack block, with the words of plan.md § The page: what steers, and why when it is not
  the subs, the readout to 0.1″, 0.1 px and 0.1 s, the longest exposure and what a stack
  started now would take, why exposures are paused, and a form that sends the four
  settings, the two shares under a `Tuning` heading (owner approval 7).
- **FR-028**: `DESIGN.md` SHALL describe the block, `.impeccable/design.json` SHALL list
  it as a component, and `crates/finder/web/dist` SHALL be the build of its source.
- **FR-029**: 035's `StackBlock` SHALL name subs dropped as `not_steady` in its
  `Dropped` line with the words `while the mount was not steady`, last, and read a
  `dropped` without that key as before (owner approval 7).

**Tuning** (US5):

- **FR-030**: The steering SHALL hold a step share and a rate share, each a number from
  0 to 1, 0.5 and 0.3 when built, and SHALL hand both to the tracker through `tune`
  before every `gap` it asks for. The tracker SHALL use them from its next gap on in
  place of FR-008's 0.5 and FR-009's 0.3, each held to 0 to 1 and a number that is not
  finite ignored; a tracker nobody has tuned SHALL use 0.5 and 0.3. The view SHALL carry
  `step_share` and `rate_share` (owner decision 3).

**The MCP tool** (US11):

- **FR-031**: The console's MCP server SHALL offer `set_steering`, which takes the body
  of `PUT /api/imaging/steering` and answers the imaging view, a refusal of the route's
  as a tool error with the route's words, and `this console takes no imaging frames` at
  once without imaging. `crates/finder/README.md` SHALL name it among the imaging
  camera's tools, and SHALL say there that `get_imaging_camera` carries the `stack` and
  `steering` parts (owner decision 11; owner approvals 3 and 8).

### Key Entities

- **Tracking error**: how far the sky at the set point is from the target, east and
  north in arcseconds, at a stated time.
- **Set point**: where on the imaging camera's sensor the run wants the target; the
  centre unless named.
- **Gap**: the time between two exposures of a stack, the only time a correction is
  made.
- **Step**: what a gap moves the telescope by, on the sky.
- **Drift**: how far the target moved on the sensor during one sub's exposure, a
  distance in arcseconds or pixels; what the readout shows.
- **Drift rate**: how fast an axis's tracking error grows between two subs once the
  steps made between them are taken out, in arcseconds a second; what the tracker
  takes 0.3 of out of that axis's rate.
- **Steering**: the console's part between the stack and the tracker; it holds the set
  point, the four settings, the exposure a stack holds and the readout.
- **Pause**: a time a stack takes no exposure because the tracker is not steady or its
  limit is under half the exposure held; the view says which with one word.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited. The only edit
  to an existing file under `crates/finder/tests` is one `mod` line for each new
  module, in alphabetical order, in `crates/finder/tests/console/main.rs`,
  `crates/finder/tests/motion/main.rs` and `crates/finder/tests/mcp/main.rs` (owner
  approval 9).
- **SC-003**: On a simulated mount that runs 1% fast, with tracking errors arriving a
  sub late, the drift within a 5 s sub is at most 0.15″ once the subs steer (from the
  12th sub in US3-S1, the 14th in US4-S5 and US7-S1), against 0.75″ without.
- **SC-004**: Every test of 015 passes unchanged, and neither
  `crates/finder/src/imaging/steering.rs` nor 035's
  `crates/finder/src/imaging/stack/service.rs` holds alt-az arithmetic (US2-S6,
  US7-S6).
- **SC-005**: While a stack runs with the switch on (owner decision 8), the steering not
  standing back (FR-014) and the tracker steady (US6-S5), no rate is sent during an
  exposure the stack keeps other than each axis's feed-forward rate plus the tracking
  offset the last gap left, whether the subs steer or have lapsed (US3-S7, US3-S11,
  US9-S11, US4-S5). A time the tracker is not steady drops the claim, and from its end
  to the stack's next gap the tracker's own corrections run as 015 has them; the sub
  that time touched is dropped, and no kept exposure begins before that gap (FR-021,
  FR-022).

## Assumptions

- **Earlier features are in place.** 015 and 038 have landed. 030, 031, 032, 033, 034
  and 035 are drafts (plan.md § Prerequisites). 031's draft carries the camera's lease
  of the plan document's amendment 1 as its US9, in the names 033 and 035 use; this trio
  reaches it through 035 (plan.md § What 035 must provide). Every `path:line` here was
  read at 49bb210, and read again there after the audit.
- **Out of scope:**
  - an equatorial mount, a tracker that corrects in small steps, meridian flips,
    dithering and guiding with a second camera (the plan document, § Deferred);
  - moving the set point: this spec keeps one and lets 037 or a later spec name it;
  - telling a wrong imaging solve from a right one, and star-shape gates (035);
  - changing a stack's exposure while it runs, and weighting subs by exposure (035);
  - stopping a stack when tracking ends, and the run's states (037);
  - retuning 015's gains for the finder camera, and any change to GoTo;
  - measuring a mount's backlash any other way than 015 does;
  - persisting the four settings (owner decision 3);
  - changing the words `get_imaging_camera` gives itself in the tool listing, which a
    test of 031's pins (owner decision 11).
- **Outside the factory:**
  - GoTo and Track proven on the real mount (the plan document, § Before any flip, 4),
    and its backlash measured on both axes (azimuth reverses for a target that
    culminates north of the zenith), before this spec is used at the telescope;
  - the field checks: drift per sub with the switch on and off, on a bright field;
    drift per sub while subs are being dropped, the finder camera correcting in the
    gaps; a run through the meridian; whether a half-second pulse of one count moves the
    tube, and the tube's motion per commanded pulse of 1″, 10″ and 30″; how late a
    tracking error arrives with the real solver at the camera's full resolution, and the
    worker's time for one sub (035's `took_ms`) against the sub cadence; Track left on
    for 30 minutes with the scope node's journal read afterwards for
    `mount command failed`, each of which is a stop of both axes that the console is not
    told of; ten minutes of Track with the axis counts read once a second, to see
    whether the axes advance at the rate sent or fall behind by a stop at every rate
    change; how long the first reversal's measurement takes to close with the real
    finder camera's scatter; and the two shares of owner decision 4 confirmed or retuned
    on the night with the tuning settings (FR-030) and then written into the code, with
    the constants of owner decision 5, which only a rebuild changes;
  - the Impeccable review of the block, after US8 lands;
  - rebuilding the console on the console machine.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002]
US2:
  depends_on: []
  implements: [FR-003, FR-004, FR-005, FR-006]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-007, FR-008, FR-009, FR-010, FR-011, FR-012]
US4:
  depends_on: []
  depends_on_merged: [US1, US2, US3, US9]
  implements: [FR-013, FR-014, FR-015, FR-016]
US5:
  depends_on: []
  depends_on_merged: [US3, US4]
  implements: [FR-017, FR-018, FR-019, FR-020, FR-030]
US6:
  depends_on: []
  depends_on_merged: [US4, US5]
  implements: [FR-021, FR-022]
US7:
  depends_on: []
  depends_on_merged: [US5, US6, US10]
  implements: [FR-023, FR-024, FR-025]
US8:
  depends_on: []
  depends_on_merged: [US6, US7]
  implements: [FR-026, FR-027, FR-028, FR-029]
US9:
  depends_on: []
  depends_on_merged: [US3]
  implements: []
US10:
  depends_on: []
  depends_on_merged: [US6]
  implements: []
US11:
  depends_on: []
  depends_on_merged: [US8]
  implements: [FR-031]
```

Every edge is a merge edge, because each dependent builds on what its predecessor
creates. US3 implements US2's `Steered` for the tracker. US9 tests US3's tracker: it
writes no code of its own and implements no FR, and it follows US3 on the chain because
both add a `mod` line to `crates/finder/tests/motion/main.rs`. US4 creates `Steering` in
US2's file over the tracker US3 made steerable, and its US4-S5 reads US1's memory; it
names US9 as well as US3, as the next story after US9 to write that `main.rs`. US5 adds
to US4's `steering.rs` and `imaging/mod.rs` and to US3's `tracking.rs`; US6 adds to
`steering.rs` and `tracking.rs` again, after US5, and to `session.rs`, which US4 wrote:
US6 names US4 for that file, though it already follows it through US5. US10 tests US6's
tracker as US9 tests US3's, with no code and no FR, and follows US6. US7 wires the
finished steering into 035's stack and writes the README after US5; it names US10 as
well as US6, as the next story after US10 to write `crates/finder/tests/motion/main.rs`.
US8 adds the readout's figures to `steering.rs` after US6 and shows them after US7's
block of the stack exists, and names them in the README section US7 writes. US11 adds
the MCP tool last, after US8, the story before it to write the README. Where a story
names two predecessors and one already follows the other, the second edge is there
because both wrote a file the story writes.
US1 touches no file in common with US2 or US3 (`crates/simulator` against
`crates/finder` and `GLOSSARY.md`) and may run beside them.
