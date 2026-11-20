Not given to builders or the judge. Nothing here is a rule; the trio is.

# History: Tracking Steered by the Subs

How `036-tracking-from-subs` came to be as it is. The trio holds the rules and flip.md
what the flip settles.

## The re-anchor (2026-11-20)

After 035 landed, the trio was re-anchored to `main` at `52785403` (flip.md
§ Prerequisites 1, 2, 3, 7 and 10, now met). The state stays `draft`, and nothing a
story builds changed.
- **Citations.** Of the 79 distinct `path:line` the trio cited, read at 49bb210, 48 were
  unchanged, 27 had moved and were renumbered, and 4 were found by their content: the
  **Imaging angle** entry (`GLOSSARY.md:97`, a line of the then uncommitted rewrite,
  now 107), the end of `Shared::pointing`'s block (`tracking.rs:313`, now 355), the loop
  that sends the rates (`tracking.rs:868`, now 940) and the last route line of `lib.rs`
  (`:35`, now 50). 21 were added where the trio named a plan.md section of 030 to 035
  or a line by its content: 18 in files those specs created.
- **Names.** Every name the trio takes from 030 to 035 is as their code has it. The
  sentences that called them drafts, or took an interface from their plan.md, now say
  they have landed and cite their code.
- **plan.md § What 035 must provide** kept what this spec's stories use: the worker's
  settings test, from its point 6, and the worker's queue of four, which was added. The
  rest of its nine points and its table of changes to 035's files serve 040's US7, US8
  and US11, and 040's plan.md § What 035 provides holds them. Each was read in the
  landed code and holds as 040 has it: the taker's loop (`Stacker::take_subs`), the
  stopping mark, the run's `Lease` and its `set`, the lease's refusal of
  `Imaging::set`, the taker's returns, the worker's `process` and `run`, `Dropped`
  with `count`'s `unreachable!`, `StackBlock`'s `Dropped` line and `Stacker::start`'s
  refusal of a camera not `ready`.
- **Shared writes.** All eleven that name 015, 030 to 035 or 038 matched `git log -1` on
  `main`; their reasons now name the commit.
- **Principle V's row** names PHD2, with a row in § Decisions and the alternatives
  rejected.
- **A stale count.** `STALE_GAPS`'s doc comment said three; the constant is eight.

## The split (2026-11-20)

The spec held eleven stories when it was split in two:

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

Where the sections of the spec as it stood went:

| Before the split | 036 | 040 |
|---|---|---|
| spec.md comment header | below, and a new header of 7 lines | a new header |
| spec.md § Clarifications | below, whole | — |
| spec.md § Owner decisions | flip.md: 1 to 8, 13 to 16 | flip.md: 9 to 12, 17 |
| spec.md § Owner approvals | flip.md: 1, 2, 3, 4, 5, 8 (the route row), 9 (its modules) | flip.md: 3 (`set_steering`), 6, 7, 8 (README section, MCP lines), 9 (its modules) |
| spec.md § User Scenarios, definitions | all | all but the axis displacement and the star centroid |
| spec.md § Edge Cases | 18 bullets | 11 bullets, and one written for it: the limit under half |
| spec.md § Requirements | FR-001 to FR-020, FR-030 | FR-021 to FR-029, FR-031 |
| spec.md § Key Entities | all but **Pause** | **Pause** |
| spec.md § Success Criteria, Assumptions, Work Graph | written for the piece | written for the piece |
| plan.md § Summary to § Project Structure | written for the piece | written for the piece, and § What 036 provides |
| plan.md § The simulator's memory, Count steps, The tracking error, `GLOSSARY.md`, The tracker | whole | — |
| plan.md § The tracker's two answers | US5's part | US6's part, as § Steady, the tracker's second answer |
| plan.md § The steering | US4's and US5's | US6's and US8's, as § The steering's wait and its check |
| plan.md § The session | US4's | US6's push, as § The session's push |
| plan.md § The imaging state, The route | whole | — |
| plan.md § The stack, `set_solver`, README, The page, `DESIGN.md`, The MCP tool | — | whole |
| plan.md § What 035 must provide | whole | the points US7, US8 and US11 rely on, as § What 035 provides |
| plan.md § Shapes that existing tests pin | the shapes of US1 to US5, What 037 reads | the shapes of US6 to US11 |
| plan.md § Decisions and the alternatives rejected | below, 036's rows | 040's history.md, its rows |
| plan.md § Prerequisites | flip.md | flip.md, a list of its own |
| plan.md § Plan traps | 29 traps, numbers kept | 27 traps, numbers kept |
| plan.md § Risks | 20 rows; one row below | 15 rows |
| plan.md § Refinement Record, its prose | below | — |
| tasks.md | phases 1 to 5 and 9; its prerequisites to flip.md | phases 6 to 8, 10 and 11 |

Where a rule was written differently in several places, the split chose none of them;
the split's report lists the differences it found.

## The spec's comment header before the split

```text
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
```

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

## Story notes moved out of spec.md

**User Story 3**, under its story:

With round 1's cases and the reanalysis's folded in, this story's estimate passed
60,000 B of the 64 KiB budget, so by the owner's choice 14 of 2026-11-19 thirteen
scenarios are User Story 9's: US3-S8, US3-S12 and US3-S13 whole, and the later halves of
US3-S2, US3-S3, US3-S5, US3-S6, US3-S7 and US3-S11, which each held more than one Given,
When and Then. What they test is still this story's code. The scenario that was US3-S14,
the most one gap changes a rate by, took the freed number 8, so that US3-S9, US3-S10 and
US3-S11 keep theirs (§ Clarifications, the session of the reanalysis).

**User Story 9**, under its story:

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

## Asides moved out of the trio

Each was replaced in the trio by the rule or the reason it gave, without the date,
the probe round or the earlier draft it told of. As each read:

- **spec.md US2-S6, its oracle**:

  > (oracle: the plan document's rule 1; the last seven names were added on
  > 2026-11-19, since a file could have said any of them and passed).

- **spec.md US5-S9, its oracle**:

  > (oracle: FR-019. Turned off while a stack runs, the switch lets the tracker go at
  > once, so that its own corrections steer; as drafted the claim stood, no gap came to
  > lapse it, and nothing steered).

- **spec.md US9-S12**:

  > taken again, as 035's worker hands over a sub taken before the push. −600″ east is
  > such a sub's error against the new place; the recheck of round 1 ran that case
  > before the rule was written and saw a step of (7.49, 0.02)″).

- **plan.md § The tracker, after a push**:

  > has claimed again. The
  > recheck of round 1 ran that case and saw a step of 7.5″ where nothing should move (EE2).

- **plan.md § The tracker, The rate task step 4**:

  > rate's time can run out before the next turn. Round 1's fix sent `base` only when
  > `proportional` was not zero and `proportional_until` not yet reached, and missed
  > both: the recheck of round 1 measured +25.3″/s and +27.8″/s on the azimuth axis still
  > in force at the gap's answer for a pointing with no earlier one, where round 1 itself
  > had measured +0.632″/s for a pointing 40 s after the one before (D25, EE1). US10-S1,

- **plan.md § The tracker, the integral**:

  > as the subs left it. Held and summed, as this trio first had it, one gap added the
  > changes of all its pointings at once, each made for the same uncorrected error, so the
  > loop's gain grew with the square of the gap: past about 25 s between gaps it was
  > unstable, and on a finder scatter of 15″ it moved the offset by 0.2″/s to 0.5″/s a gap,
  > which spoils the rate the subs had learned and smears the very sub that would take the
  > tracker back (reanalysis AA1, FF4, from models of this section; round 1's simulator,
  > with 6 s gaps and the truth as its finder pointings, did not show it).

- **plan.md § The steering, standing back**:

  > The simulator's aim point is exact, so no rig of round 1 met it (reanalysis BB5;

- **plan.md § The steering, the exposure held**:

  > pass 1 s, or the camera's shortest where that is longer: as drafted it stopped only at
  > the camera's shortest, and a 30 s setting near the zenith became subs of a quarter of a
  > second, 130 frames of 31 MiB for each 30 s asked for.

- **plan.md § The steering, Letting the tracker go**:

  > nothing steers at all. As drafted `set_on` only stored a flag, and a switch turned off
  > during a stack, which is how the field check with and without the feedback would be run,
  > compared the feedback with a mount nothing corrected (reanalysis AA5, CC1, DD5, FF2;
  > AA12 for the set point).

- **plan.md § The steering, the view's `longest_exposure_s`**:

  > made, like `steered_by_subs`. As drafted it was kept from the latest
  > `before_exposure`, so before a stack the page showed no limit, and after one, or with
  > Track off, a limit an hour old (reanalysis FF17);

- **plan.md § What 035 must provide, its opening**:

  > 031 has landed with the camera's lease, and 035's eleven stories have landed on it: its
  > US9 and US10 on 2026-11-19, after § The stack was read against `main`. They wrote the
  > page, the README and the MCP tools, and `stack/service.rs` is still as its US8 left it.

- **tasks.md T010**:

  > feedback beside noisy finder pointings (US3-S2). Thirteen scenarios that were this
  > story's are US9's since 2026-11-19: this module needs no rig built without the wait
  > and no goto console.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| A step in the gap and a rate from the drift between subs | 015's proportional-integral law with other gains | Its proportional term is a rate held until the next solve: a 2″ error is about 1″ of deliberate drift in the next sub. Its integral winds up while a first offset of tens of arcseconds is taken out, and unwinds over several smeared subs. Here the step takes the offset and the rate takes only the drift rate, so the two never work against each other on a mount that makes the step asked for; for one that does not, the next two rows. |
| The rate from the difference of two errors, less the step made between | The rate from the error itself | The difference measures drift whatever the offset; the tracker knows the step it made. |
| No rate change when the drift rate is over twice the offset's limit, and at most 0.1″/s of change in one gap | Always; the jump test alone | A jump that large is a wrong solve, a gust or a bump, not a rate error the offset could hold. But the test is of a rate, so the jump it lets through grows with the sub length: 9″ at 6 s between subs, 47″ at 31 s. A smaller one is believed, and 0.3 of it went into the offset; 0.1″/s a gap bounds what one bad error costs at any sub length, and a 1% rate error, whose first change is 0.06″/s, converges as before (spec owner decision 4, changed 2026-11-19). |
| No rate change from a pair of errors with more than two deadbands of steps between them | Believing the step asked for was the step made | The simulator makes every step exactly. A real pulse is two rate commands half a second apart over a serial link, and a 10% error in a 30″ step read as 0.5″/s of drift, inside the jump test. The rate now waits until the steps are small (spec owner decision 4, changed 2026-11-19). |
| A step booked for the time its pulse really lasted | A flat half second | On the simulated link a pulse lasts exactly 0.5 s. On a real one it lasts as long as its two commands take as well, and the unbooked part would be read as drift. |
| The deadband's 1″ and the step's 30″ as angles on the sky | Angles of the axis | An axis angle is worth less sky toward the mount's pole: 30″ of azimuth is 5″ of sky at 80° of altitude, and 1″ of it is under any solve's noise (spec owner decision 5, reworded 2026-11-19). |
| A deadband of two counts or 1″ on each axis's step | One figure in arcseconds; one count | A step of under a count cannot be commanded; half of two counts is one. 1″ keeps a fine-geared mount from chasing solve noise. The tracker reads the counts from the mount, so nothing is sized to one. |
| The step as a half-second pulse inside the gap | A proportional rate over the next exposure | Nothing but the feed-forward then changes while the shutter is open, however late the next gap is. |
| Gaps served by the rate task | `gap` sending rates itself, under a lock | The scope node accepts a rate that arrives after a Stop. Only the rate task is ended before Stop is sent (`crates/finder/src/tracking.rs:885`). |
| Freshness counted in gaps | An age in seconds | The plan document's rule 6; a sub may last minutes on another mount. |
| The finder camera changes no rate while the subs steer | A finder error over some bound takes the tracker back | Its scatter is the fault being removed; a real loss of the target shows as subs that do not solve, which hands the tracker back after eight gaps. |
| The steering stands back from a stack whose first tracking error is over 180″ | Steering it in; holding the field wherever the stack began | The finder camera holds the target on its aim point and the subs on the set point. Where they differ by more than 038's 5′, the subs' centring reads as a push by hand and Track follows it, again and again. 180″ leaves room under 300″ for the finder camera's scatter. Holding the field where it began would change what `set_set_point(None)` means for 037 (spec owner decision 7, added 2026-11-19). |
| Eight gaps with no tracking error before the claim lapses | Three; counting sub verdicts | 035 keeps four subs waiting and one in work, so with a slow worker errors come several gaps apart with nothing dropped; at three the claim lapsed between every two and no drift rate was ever measured. Counting the worker's verdicts exactly needs a third call from 035's worker and more code in the largest story (spec owner decision 6, changed 2026-11-19). |
| An error measured before a follow, or before the tracker stopped being steady, is dropped by its time | Forgetting only what is pending at that instant | 035's worker hands a sub over one or more subs late, so the error can arrive after the claim has been taken again. |
| The cap as a turn of the field in radians | Passing pixels and a sensor size to the tracker | The tracker then needs nothing of the camera, and an equatorial tracker answers "no limit" with no arithmetic. |
| Halving the exposure | Following the limit to the millisecond | 034 matches a master dark within 1 ms of the sub's exposure; a few exposures can each have one. |
| A route of this spec's own | Keys of 031's settings | 031's tests pin the settings body and the MCP tool to nine keys. |
| `Steering` as a part of its own, with a source closure | The stack calling the tracker | One place holds the set point, the settings and the readout; a test gives it a scripted tracker; 037 reads it. |
| The simulator remembers its telescope's direction | The simulated solver keeping its own record | The truth is the simulator's to say; 032's adapter and every later test then need no change. |
| Count steps as a setter | A field of `MountConfig` | Tests build `MountConfig` by literal. |
| Each error judged against the gaps since its mid-exposure | Applying each error as if it were measured now | 035's worker solves a sub while the next is exposing, so every error is at least one gap old. Applied as fresh, each offset is stepped out twice and the rate loop runs with a delay, which rings. Remembering the last 16 gaps costs a small list. |
| Only an accepted sub steers | Every solved sub | The plan document's own words. A solve 035 refuses never reaches the mount; the cost, that a sub dropped as soft steers nothing, is spec owner decision 15. |
| The exposure decided once and held | Following the limit sub by sub | 035 drops a sub whose exposure differs from the reference sub's, and a take exposes for the console's setting. |
| `limited` decided with the exposure and held with it | Comparing the held exposure with each call's wanted one | A later call may ask for less than is held; whether this stack's exposure was cut does not change with what is asked later. |
| A lapsed claim is taken back only once a tracking error has come | Taking the tracker back at the next gap | While subs are dropped, the next-gap rule leaves the finder camera steering during one exposure in nine and nothing steering during the other eight. This one hands the tracker to its own corrections whole (spec owner decision 6). |
| After a lapse, the finder camera's step held for the next gap and applied there, and the tracking offset left as the subs set it | 015's corrections at once, as written; the integral's changes held too and added up | The settled row says rate corrections change only between exposures, and a stack that has lapsed is still exposing: a sub taken then may be accepted and is the one that takes the tracker back. Held, every sub's exposure sees only the feed-forward and the offset. The integral is not held: summed over a gap's pointings its gain grows with the square of the gap, it is unstable past about 25 s between gaps, and it writes the finder camera's scatter over the rate the subs learned (spec owner decision 6, changed 2026-11-19). |
| A gap served under a lapsed claim always sends each axis its rate | Sending only when the offset changed, or when `proportional` says a rate is in force | `Control` cannot say what the mount was last sent: a later pointing can zero `proportional`, and its time can run out, with the rate still on the mount until the next turn. The recheck of round 1 ran the rule with the whole workspace passing. |
| A gap before a tracker's first rates is turned away | Serving it like any other | `direction` is not set until the task's first turn, so `commanded` would send both axes zero. From US6 such a tracker is not steady and the same gap claims nothing, so the two stories would disagree about every scenario that began with it. |
| The steering keeps weak references to trackers | Strong ones | Dropping a `Tracker` stops the mount (`crates/finder/src/tracking.rs:112`); a record must not put that off. |
| A sub is matched to its exposure by its mid-exposure time | A ticket carried through 035's queue | It needs no change to 035's hand-over, and half an exposure of margin either side is far more than the clocks' mapping error. |
| `source` says `tracker` | `finder` | The plan document's rule 5: another telescope may have no finder camera. |
| The exposure decided from the tracker's shortest limit of the hour ahead | The limit at the instant the stack starts; the wait alone; ending the stack with a reason of its own | A stack holds one exposure for life, and before its transit a target's limit only falls: a stack asked for 30 s on M31 an hour before transit held 7.5 s and trailed 12 px at transit, at the default smear. An hour, sampled each minute, is what 015's own forecasts look at (spec owner decision 2, changed 2026-11-19). |
| No halving to under 1 s | Halving down to the camera's shortest | Near the zenith a 30 s setting became subs of a quarter of a second, 130 frames for each 30 s asked for, most of them dropped as `behind`. |
| A default smear of 6 px | 2 px | The 2 px was sourced nowhere and cut a 20 s setting on M42 to 5 s. The research note gives 28 px of coma in the same corner, a published 7 px, and the owner's 20 s to 30 s subs with this camera (spec owner decision 1, changed 2026-11-19). |
| The limit asked as the view is made, with what a stack started now would take | Kept from the latest `before_exposure` | Kept, it was absent before a stack, when it decides which darks to take, and stale after one. |
| `view_for` beside `view` | Changing `view`'s signature; keeping the ask in the steering | US4's tests call `view()`, and US5 may not edit them. The ask is the camera's and the settings', which `Imaging` holds. |
| The two shares as run-only settings, handed to the tracker before every gap | Constants in code; a second argument of `steer` | The telescope is two hours from where a console is rebuilt, and the switch alone compares all of the feedback with none. Handed before every gap, a tracker a GoTo has just made has them by its first step, and `steer` keeps its shape. |
| The switch turned off, a set point that changes and the wait for the limit each let the tracker go | Storing the flag or the set point only | A claim lapses only at a gap, and none is asked for then: the claim stood, the finder camera's pointings changed no rate under it, and nothing steered. |
| US3 and US6 cut by tests, into US9 and US10 | Building at 93% and 92% of the budget; the code cut § Risks once named for US3 | A cut by tests moves whole scenarios and no line of code, so the story that keeps the code is graded on fewer criteria and builds as it would have. The code cut was never built. A cut found mid-epic needs a relaunch. |

Added at the re-anchor of 2026-11-20, for principle V of constitution 1.3.0, which asks a
plan to name what it looked at and why it turned it down:

| Decision | Rejected | Why |
|---|---|---|
| The correction from the subs is this project's own code | PHD2 (BSD-3-Clause), the proven guider | It guides a mount through ASCOM, INDI or ST-4, and this mount is driven by the scope node, which has no INDI mount driver. It expects a guide camera's steady stream, not imaging subs that may come minutes apart. Nothing of it is copied. |

## The Refinement Record: how the trio was drafted, probe-built and folded

Drafted on 2026-11-15 by the grilling session, from the plan document, a read of
`crates/finder/src/tracking.rs`, 015's trio and tests, the simulator, and the drafts of
030, 031, 032, 034, 035 and 037. A first drafter wrote the trio against `main` at dbc2b5b
before 035 had a draft and was cut off. A second finished it the same day against
`main` at 49bb210: it re-read every cited line there; took 030's, 031's, 032's and 034's
interfaces from their reviewed plan.md; read 035's draft plan.md, which changed US4 to
US7 (two tasks and a queue, so tracking errors arrive late, a sub is checked before it
is solved and steers only once accepted, and the exposure is held for a stack's life);
brought the words in line with the plan document's § Mount-neutral by design; and
corrected two scenarios (US3-S5, US4-S5) whose step would have been cut by the rule
that no correction reverses an axis. A second agent then reviewed the trio, and its
twenty-three findings were each checked against the files, the code and the research
and applied the same day, against 035's revised draft (the camera's lease,
`lease.take`, its tasks.md and its Refinement Record) and 033's. The changes that
reach across stories: the cap is set through the stack's own lease and a refused set
ends the stack (US7, § The stack, § What 035 must provide); the tracker rig waits for
a tracker's first rates and a gap before them is turned away (US3, US6, § Plan traps
22); a lapsed claim is taken back only once a tracking error has come (US3); `limited`
is held with the exposure (US5); and six scenarios whose numbers could not pass as
written were corrected (US2-S5, US5-S1, US5-S2, US5-S6, US7-S1, US8-S4). An audit of
the seven trios the series then had, side by side, followed the same day, and its five findings
were applied (spec.md § Clarifications, the audit's session): after a lapse the
finder camera's corrections are held for the next gap, and while the tracker is not
steady they run as 015 has them (US3-S6 [its lapse now US9-S4], US3-S11, US6-S5, § The
tracker, § Plan traps 27 and 28); the stopping mark is named, and the observer's
exposure is put back where the taker returns (§ The stack, § What 035 must provide);
rule 1 is no longer to be edited (§ Prerequisites 9); US7-S6 guards 035's
`stack/service.rs`, and owner decision 1's widening is worked; and a bound on the rate
one tracking error may ask for waited on 032's owner decision, which the owner settled
on 2026-11-17 with no bound (§ Prerequisites 11). Every `path:line` in spec.md
and plan.md was then read again at 49bb210, the worktree's HEAD; none had moved. **Until
probe round 1 no probe build had been run:** no story had been compiled, and no scenario
had been seen to pass or fail, before the review, after it, or after the audit.

What stood in for a probe until round 1, and what it did not show:
- **A model of the loop**, worked by hand for this draft: two axes at M42's rates for
  2026-01-16T02:00Z at 40° N (azimuth 19.9″/s, altitude 3.6″/s), a 1% rate error, 5 s
  subs, errors read at mid-exposure. With each error judged against the gaps since its
  mid-exposure, every drift measured is the rate error as it then stands and 0.3 of it
  is taken, so the drift within a sub falls by 0.7 a sub from the first drift measured:
  0.749″ at first and 0.030″ at the 12th sub when errors arrive a sub late. The spec's
  bound of 0.15″ is that figure with margin. The model has no once-a-second refresh, no
  link, no pulse shape and no real axis resolution, and it was not run as a program.
- **The finder-noise scenario** (US3-S2) rests on arithmetic alone: a 15″ scatter on
  each coordinate every 2 s puts a proportional rate of about 9″/s on each axis for
  most intervals, which is tens of arcseconds in 5 s.
- **The held correction** (US3-S11) was worked by hand from 015's constants and a
  spherical-trigonometry check of M42's place at the rig's start (altitude 43.0°,
  azimuth 161.8°, an hour angle of −13.3°): 30″ east is −39.8″ of azimuth and −7.2″ of
  altitude, so the step is about +23.9″ and +4.3″ and runs with both axes. US6-S5's
  measurement under a lapsed claim mirrors 015's own `measured` helper
  (`crates/finder/tests/support/tracker.rs:323`) and had not been run.
- **The worked numbers** of US2, US4-S4, US5 and US8 were recomputed with a short
  script; the limit of 37.15 s and the 65″ of refraction at the rig's start likewise
  (probe round 1 measured the limit on the rig as 37.28 s, at 14.91°/h, 0.36% off: D32).
  The review recomputed them independently, and its corrections are in: the turns of
  US5 are now the test's own `hypot` expressions, US3-S10's signs are right, US5-S6's
  due-east instant is worked out (9.7″ of azimuth a second against a 3.2″ window), and
  US8-S4 no longer rests on how a tie rounds.
- **Anchors** were read at 49bb210, read again there after the audit, and checked by
  the validator.

Before round 1, for the probe build and the re-anchoring: every bound named in
§ Prerequisites 5; whether `serve_gaps` can be woven into `run` as described without
disturbing the keyhole's `second.reset()`; whether a gap that takes 0.5 s lets the next
once-a-second turn slip past 015 US3-S1's "at least once a second" in any existing test
(no existing test calls `gap`); US3's test file under US6's test in `serve_gaps` step 1
(the tracker rig waits for the first rates, and US3-S7 [now US9-S7] pins what a gap
before them does, so the two stories should agree, but only a build of both shows it);
whether the wait for the first rates ever takes more than a few milliseconds of virtual
time; US3's test file under US6's held-mode test, `lapsed && control.steady`; whether
US6-S5's follow keeps 015's 60″ bound with the finder camera's corrections held for gaps
6 s apart; US7-S5's rig; and all of § Prerequisites 3.

**What probe round 1 changed** (2026-11-18 and 19; spec.md § Clarifications, the session
of round 1 probes). Four agents built the eight stories from this trio, in order, on
probe builds of 030 to 035, tests first and with both gates, and found 3 blocking and 44
clarity defects, D1 to D47, all folded on 2026-11-19 with the owner's answers of that
day. Of the questions above, all but § Prerequisites 3, which waits for 035 to land,
were answered: every bound held (§ Prerequisites 5); `serve_gaps` was woven into `run`
as described, with the target and counts read in its step 1 (D9); 015's tests, and every
other existing test but the one 034 test of D43, passed under both gates on every story;
US3's test file passed unedited before and after US6; US6-S5's follow kept 015's 60″
bound; and US7-S5's rig ran. The changes that reach across stories: US6-S1's gap answers
at L or after it (D24), US7's drift oracle reads each saved sub from its FITS file
(D37), and the stacking rig waits 1 s more before a stack starts (D38), the three
blocking defects; a lapsed gap that zeroes a proportional rate still in force sends the
zeroed rate (D25: `serve_gaps` steps 2 and 4, FR-010, US6-S1 [now US10-S1]); 038's
`follow` ends the claim as `release` does (D6: FR-011, US3-S12 [now US9-S12],
§ Prerequisites 12), and 038 joins `depends_on_landed`; the owner trigger of
§ Prerequisites 3 is narrowed to a comparison
with a literal or a fixed list of keys (D8), and that prerequisite gains a check of the
regions read from the page's end (D43); three scenarios are new, US1-S6, US3-S12 [now
US9-S12] and US4-S7; every case the round proposed where a mutation survived is in its
scenario, in the round's words; and § What 035 must provide and § The stack gain three
words (D39 to D41). The records below change with them: new tests for US1-S6, US3-S12
[now US9-S12] and US4-S7 and widened assertions where cases were added; US3's shared
write of the tracker names 038's US1, and its preserved behaviour 038's tests; US7's
preserved tests name 035's `stack_camera.rs`; and US8 writes the README, after US7's
section. No story was over 64 KiB, and none is on the folded figures (§ Risks).

**What the reanalysis changed** (2026-11-19; spec.md § Clarifications, the session of
the reanalysis). Six reviews of the folded trio found 78 more issues, AA1 to FF20, and
the owner took the recommended answer to all fifteen choices they came to. They were
folded the same day in two passes; § Prerequisites 5 says what was run and what was not.
The first pass changed the tracker and the stack's wiring: a push the session reads as
moving, 2 s of settling and a turn that finds an axis stopped are times the tracker is
not steady (FR-021, US6-S6, US6-S7); the steering stands back from a stack whose first
tracking error is over 180″ (FR-014, US4-S8, US7-S7); the claim lapses after eight gaps,
and a lapse holds the finder camera's step and not its integral (FR-011); no gap changes
a rate by more than 0.1″/s, a pair of errors with more than two deadbands of steps
between them changes none, and a step is booked for the time its pulse lasted (FR-008,
FR-009); an error from before a follow or a time not steady is dropped by its time, and
a lapsed gap always sends each axis its rate (FR-010, FR-011); and US7's calls are
written for the 035 that landed (§ The stack). It added the edge from US6 to US4, for
`session.rs`. The second pass changed the cap, the view, the page and MCP: the exposure
is decided from the hour ahead, not halved to under 1 s, and waited on while the limit
is under half of it (FR-017, FR-018, FR-022); the default smear is 6 px; the view
carries the limit and what a stack started now would take whenever a tracker runs
(`view_for`); a pause says why, in the tracker's word (FR-021, FR-022); the page's
sentences are reworded (§ The page); the two shares are settings (FR-030);
`set_steering` is an MCP tool (FR-031); and the switch turned off, or a set point
changed, during a stack lets the tracker go (FR-014, FR-019). It split every scenario
that held a second Given, or a When on another rig, and it cut US3 and US6 by tests: US9
and US10 hold the scenarios that left them, write no code and implement no FR, and US11
is the tool. The records below change with all of it: three new blocks; 59 new entries,
each a scenario that is new or was the later half of another, whose own line is trimmed
to what it still asserts; US4's and US7's edges to US9 and US10, for
`crates/finder/tests/motion/main.rs`; US5's write of `imaging/mod.rs`; and the
development checks of US6, which run US9's module beside US3's. Two scenarios were found
broken by the fold's own rules and mended: US4-S4's set point and a case of US5-S1
(spec.md § Clarifications).

## A risk that told the record's own history

| Risk | Mitigation |
|---|---|
| The Refinement Record nears its 65,536 B. | It is the fence's text the validator measures. Each scenario costs an entry of about 300 B, and this fold's 59 new ones took it from 43,878 B to the figure the fold's report gives. The one lever left is formatting, two spaces off the record's list items; no assertion is cut to make room. |
