---
state: draft
depends_on_landed: ["015-tracking", "030-imaging-camera-link", "031-imaging-frames-on-console", "032-imaging-solve-and-aim-point", "033-focus-reading-and-aid", "034-dark-and-flat-frames", "035-live-stack"]
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
# Not yet probe-built: no story of this trio has been compiled or run. The bounds of
# US3-S1, US3-S2, US4-S5, US6-S5 and US7-S1 come from arithmetic on a model of the
# loop, not from the code.
# It builds on code that 030, 031, 032, 033, 034 and 035 create and that does not exist yet.
# Every interface of theirs named here is taken from that draft's plan.md. The trio is
# re-anchored against `main` after they land and before it is flipped (plan.md
# § Prerequisites).
# It stays draft until the owner settles the owner decisions and flips it to `ready`,
# which gives the approvals listed under "Owner approvals" and dispatches it.
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
  (owner decision 6); the switch turned off is the one declared exception (owner
  decision 8).
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
  draft carries it as its US6 (031 plan.md § The camera lease), and 033 and 035 cite it
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
- Q: What does 037 read from this spec? → A: Two things (037 plan.md § Prerequisites
  2): whether the stack is waiting for the tracker, which is `steering.waiting` in the
  imaging view and `Steering::waiting()`; and whether the tracker is steady enough to
  expose, which is `Steered::steady()` of `Session::steered()`. It also makes one call:
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
     offset it would have made, and the next gap applies them (owner decision 6,
     FR-010, FR-011, US3-S6, US3-S11); with no stack, 015's behaviour is unchanged.
     Owner approval 2 now covers every state in which gaps are served. While the
     tracker is not steady its own corrections run as 015 has them even after a lapse,
     because gaps are turned away then and 015 closes a first reversal's measurement
     from them (FR-021, US6-S5). The switch turned off is written as the observer's
     declared exception (owner decision 8).
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
     (FR-023). Owner decision 1's "about 5%" was in no source; it is now worked there
     from the research note's in-focus star size.
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

## Owner decisions

Each is a design or taste call the plan document leaves open. None is settled. A
different answer is written into the trio and re-validated before the flip, never left
to a story.

1. **The smear's unit and default.** Recommended: pixels of the imaging frame as
   delivered (after binning), at its corner farthest from the set point, 2 px by
   default.
   It is what 015's readout uses and what the eye judges in a sub; 2 px widens an
   in-focus star by 3% to 4% along the trail (worked here; no source gives the figure:
   the research note puts in-focus stars at this telescope at 4.6 to 5.5 px,
   `docs/research/2026-11-15-imaging-camera.md` § 6. Focus, and its half-flux diameter
   equals a Gaussian's full width, 2.3548 σ; a uniform trail of L pixels adds L²/12 to
   the variance along it, so the width grows by √(σ² + 4/12)/σ: σ = 1.953 px and a
   factor of 1.043 at 4.6 px, σ = 2.336 px and 1.030 at 5.5 px); and it needs no scale,
   so the cap can be decided before a stack's first imaging solve, on a console that
   has measured nothing yet. Two things speak
   against it and are the owner's to weigh. The research note records that a limit in
   arcseconds does not change with binning while one in binned pixels doubles
   (`docs/research/2026-11-15-imaging-camera.md` § 7. Field rotation, sub length and the
   drive): at binning 2 the same 2 px is twice the smear on the sky. And the plan
   document's rule 3 says "Thresholds are in arcseconds or relative to a measured
   value". A smear in delivered pixels keeps what rule 3 is for: it holds no focal
   length, pixel size or scale, and what the tracker is asked is a pure angle, the
   smear over the corner's distance in the same pixels. It is not rule 3's letter.
   Instead: arcseconds on the sky, 2″ by default, turned into pixels with 032's
   measured scale or, before one, the scale from the pixel size and the focal length
   setting, and no cap when neither is known; or 1 px, 015's readout figure.
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
   dark for every length.
3. **Where the two settings live** (the plan document's route table, in this worktree,
   has this route as 036's row). Recommended: a route of this spec's own,
   `PUT /api/imaging/steering`, run-only as 031's console settings are; the plan
   document's table already has the row, and another answer corrects it (plan.md
   § Prerequisites 9). Instead: keys of
   `PUT /api/imaging/settings`, which 031's tests pin to nine keys and which the
   stack's lease refuses while a stack runs; the session file; or a flag of the binary.
4. **The control law.** Recommended: a step and a rate. In the gap after a tracking
   error, the tracker moves each axis by half of that axis's error when it is outside
   the deadband, as one half-second pulse; and it takes 0.3 of the drift rate measured
   between two subs out of that axis's rate. Both allow for what the gaps since the
   sub's mid-exposure already did, so an error that arrives late is not corrected
   twice. A large first offset then never winds the rate up, and nothing but the
   feed-forward changes during an exposure. Instead: 015's proportional-integral law
   with other gains.
5. **The deadband.** Recommended: per axis, two of that axis's counts or 1″, whichever
   is larger, so the smallest step asked for is one whole count of the controller's
   reference (1.31″ and 0.65″ on the worked example's mount). The rate has no deadband:
   a rate can be set finer than a count. Instead: one figure in arcseconds for every
   mount.
6. **When the subs stop steering, and when they steer again.** Recommended: they stop
   when the stack lets go, when three gaps in a row bring no tracking error, when the
   tracker is not steady, or when tracking ends; counted in subs, never in seconds. The
   tracking offset is kept and the tracker's own corrections, from the finder camera on
   this mount, steer again. After three gaps with no tracking error a stack still holds
   the tracker, and its own corrections are then applied only in the stack's gaps: a
   finder pointing that arrives between two gaps changes no rate, and the next gap
   applies the step 015's proportional term would have made for the newest pointing
   (its rate over its interval, 0.6 of the error) and the changes 015's integral would
   have made to the tracking offset. The tracker stays so until an accepted sub hands
   it a tracking error, and the gap after that takes it back and forgets what was
   held. So while subs keep being dropped (cloud, a wrong field) the finder camera
   steers without a break, and the settled rule "rate corrections change only between
   exposures" holds while the subs steer and after they lapse alike. Once the stack
   lets go, or while the tracker is not steady (owner decision 17), 015's corrections
   run as written: no stack is exposing then, or the stack is waiting and drops the sub
   that crossed that time (owner decision 9). Instead: take the tracker back at the
   very next gap, which leaves the finder camera steering during one exposure in four
   while subs are dropped and nothing steering during the other three; or, after the
   three gaps, 015's corrections as written, whose rate changes then fall inside the
   exposures the stack goes on taking; or an age in seconds.
7. **The finder camera while the subs steer.** Recommended: its live pointings change no
   rate, whatever they say; they still feed the pointing model, the cable watch, the
   keyhole's resume, the backlash measure and 015's readout. Once the subs have lapsed
   they change a rate only through a gap (owner decision 6). Instead: let a finder
   error over some bound take the tracker back.
8. **A switch.** Recommended: `on`, true by default, in the same route. Turned off, the
   stack hands the tracker no error and claims nothing, and the exposure cap and the
   pause still apply. The field check "with and without the feedback" needs it. It is
   the observer's declared exception to the settled rule that rate corrections change
   only between exposures: with the switch off no stack holds the tracker, so the
   tracker's own corrections run as 015 has them, during exposures as well, which is
   what the field check compares the feedback with. Instead: no switch; or a switch
   that still holds the tracker, its own corrections applied in the gaps as after a
   lapse.
9. **Subs taken across a time the tracker was not steady.** Recommended: dropped before
   they are solved, with the reason `not_steady`, including a sub during which tracking
   ended or was started again. Instead: kept, and left to 035's own gates.
10. **The readout's window and the drift's definition.** Recommended: the last 10
    tracking errors; a sub's drift is the change in tracking error since the sub before,
    less the steps made between them, scaled by the exposure over the time between their
    mid-exposures. Instead: the last 60 s; or the bare change between subs.
11. **MCP** (a departure). The plan document says "Each step also has its own route and
    MCP tool", and its list of tools has none for 036, whose `steering` part it has read
    through `get_imaging_camera` and the session tools. Recommended: no new tool.
    Tracking from the subs is not a step an MCP client starts: it runs inside 035's
    stack, which has its own route and its `set_stacking` tool, and with the defaults
    (the subs steer, 2 px) it needs no call at all. `get_imaging_camera` and the
    session tools carry the `steering` part, read-only, so a client sees what steers,
    the tracking error and the drift. The two settings are tuning, changed from the
    page. Instead: a `set_steering` tool taking `on` and `smear_px`, behind the same
    guards as the page's route, so that the rule holds to the letter; it is one more
    story after US5.
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
    document has it: a wrong solve that 035 refuses never reaches the mount, and three
    subs dropped in a row hand the tracker back to its own corrections. Instead: also a
    sub dropped as `soft` or `off_field`, whose solve is as good as any; that would let
    the subs pull back a drift that is itself what made them soft.
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
    measurement, with the subs steering through it, which changes how 015's
    measurement closes and would need 015's backlash tests proven again under it.

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
3. **`PUT /api/imaging/steering`**, the `steering` part of the imaging view and of
   the session's `imaging` object, which the MCP tools that return either carry too,
   and the module `crates/finder/src/imaging/steering.rs` (FR-005, FR-016, FR-019). The
   plan document's § Interfaces, in this worktree, names all three and no MCP tool for
   036; another answer to owner decisions 3, 11 or 14 corrects it in the same change
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
   entry and its component in `.impeccable/design.json`; and the words `not steady` in
   035's `Dropped` line (FR-027, FR-028, FR-029).
8. **A README section and a route row** in `crates/finder/README.md` (FR-020, FR-025).
9. **Each new test module is declared in an existing test file** (SC-002): one `mod`
   line a module, in alphabetical order, in `crates/finder/tests/console/main.rs`
   (`steering_cap`, `steering_error`, `steering_gap`, `steering_readout`,
   `steering_steady`) and `crates/finder/tests/motion/main.rs` (`steering`,
   `steering_cap`, `steering_gap`, `steering_stack`, `steering_steady`), as 027's layout
   asks of every finder test. No other line of those files and no other existing test
   changes.

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
  is the whole sensor (0, 0, 1236, 822), with the `start` and `exposure_s` its scenario
  gives and any path.
- **The tracker rig** is the rig `crates/finder/tests/motion/tracking/pi_loop.rs:104`
  builds, written out again in the module that uses it, with three changes: the base
  tilt is 0° in the mount and in the model; the rig keeps its `Simulator`; and it keeps
  its `Tracker` in an `Arc`, so that a second task or a `Steering`'s source can share
  it. So: a
  `VirtualClock` from Unix 1768528800 (`2026-01-16T02:00:00Z`); the target M42; a
  `SimMount` with no backlash whose zero points put its axes on M42's apparent place at
  the start, and whose truth is loaded into the model with `load_known_terms`
  (`crates/finder/src/model.rs:347`); a wrapper round the mount that logs every rate
  command with the console's time and its axis, as
  `crates/finder/tests/motion/tracking/pi_loop.rs:41` does: **the rate log**; frames
  stopped; and a `Tracker` on the console's clock at the site 40° N 75° W with a
  minimum altitude of 15°, started on M42. A scenario gives the mount's rate error.
  **The rig waits for the first rates:** after `start` answers, it looks at the rate
  log every 10 ms of tokio time until the log holds at least one rate for each axis,
  and fails if that takes more than 5 s. A tracker's first rates are sent by its
  task's first turn, not by `start` (Clarifications), and a gap asked for before them
  moves and claims nothing. Every scenario on the tracker rig begins after that wait,
  unless it says the rig is built without it. Each module that writes the rig out
  writes the wait too.
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
- **The scripted tracker** is a type of the test module's own that implements `Steered`:
  it answers a target the test sets; logs every `steer`, `gap` and `release` with the
  console's time; makes `gap` wait a time the test sets, on tokio time, and answer a
  `Stepped` the test sets; says it is steered by the subs from its first `gap` to its
  next `release`; answers the longest exposure, `steady` and `steady_between` the test
  sets; and keeps the last window `steady_between` was asked about. Unless a scenario
  says otherwise its `gap` waits 0 s and answers (0.0, 0.0), it names no longest
  exposure, and `steady` and `steady_between` are true. **A scripted steering** is a
  `Steering` whose source answers that tracker from a cell the test can empty, so that
  the test's own `Arc` is the last one; it runs on the console's `FixedClock`
  (`crates/finder/src/session.rs:41`) at 1768528800.0, or on the rig's clock where a
  scenario moves a mount. **The worked ask** is `ExposureAsk { wanted_s: 5.0,
  shortest_s: 0.001, sensor: (1236, 822), roi: the whole sensor (0, 0, 1236, 822),
  binning: 1 }`, with the `wanted_s` its scenario gives where that is not 5 s.
- **A star's centroid** in a simulated imaging frame taken with noise off is the mean
  position of the samples within 10 px of the brightest sample, in x and in y, each
  weighted by its value less the background the scenario states, a sample's position
  being its centre, (column + 0.5, row + 0.5). 030 US2-S3 holds the same quantity
  within 0.3 px of the truth.
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
   `sim.telescope().eyepiece.az` at its start, 1 s later and 2 s later, waits 30 s more
   with the axis still turning, and asks `sim.imaging_truth_at` for those three
   instants, **Then** a committed test asserts that:
   - each answer's centre is within 0.02″ of the reading taken at that instant, and its
     angle within 0.001° (oracle: the simulator's own truth, read live);
   - the eyepiece's azimuth rose by 0.0500° ± 0.0005° over the 2 s (worked example:
     0.02°/s × 1.25 × 2 s);
   - the first and third truths' centres are more than 60″ apart, so the solve's centre
     moves with the drift;
   - the star's trail in the frame is as long as the truth says. The star sits on the
     first truth's centre, so it starts at the frame's centre, (618.0, 411.0). With d
     the separation of the first and third truths' centres in arcseconds over the
     truth's scale, A the position angle from the third truth's centre to the first's
     (`atan2(sin Δα·cos δ1, cos δ3·sin δ1 − sin δ3·cos δ1·cos Δα)`, Δα the first's RA
     less the third's, east of north) and θ the third truth's angle, it ends at
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
   **When** a test reads the first's `imaging_truth()` at t and t + 2 s only, and the
   second's at t, t + 1 s and t + 2 s, **Then** a committed test asserts that the
   first's `imaging_truth_at(t + 1.0)`, asked at t + 2 s, is within 0.02″ and 0.001° of
   the second's reading at t + 1 s (oracle: a twin read live; a constant rate makes a
   straight line between samples exact).
3. **Given** the simulator of US1-S1 after 10 s of turning, **When** a test reads the
   truth at t, waits 1 s, calls `push_to(40.0, 200.0)`, and asks `imaging_truth_at(t)`,
   **Then** a committed test asserts the answer's centre equals
   `sim.directions_at(t).eyepiece`'s RA and Dec within 10⁻⁹ degrees: a push by hand
   starts the memory again, and the answer is the one 030 gives (oracle:
   `crates/simulator/src/optics.rs:120`).
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
   is 7.2″, nine counts of 0.8″).

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
   binning 1.
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
   used here), and so does the worked target with a time that is not finite.
   **Given** a type of the test's own that implements the five required methods of
   `Steered` and no other, **Then** `longest_exposure_s(0.001)` is `None`, `steady()` is
   true and `steady_between(0.0, 1.0)` is true.
6. **Given** the text of `crates/finder/src/imaging/steering.rs`, read by the test from
   `CARGO_MANIFEST_DIR`, **Then** a committed test asserts it holds none of
   `ephemeris`, `alt_az`, `parallactic`, `counts_per_revolution`, `AxisInfo` and
   `MountClient` (oracle: the plan document's rule 1).

---

### User Story 3 - The tracker steered by tracking errors (Priority: P1)

As the tracker, once a stack claims me I take my corrections from the tracking errors
it hands me: in each gap I move each axis by half of its error when that is two counts
or more, and I take 0.3 of the drift rate I measure between two subs out of its rate.
An error that reaches me late is judged against what my gaps did since it was measured.
I change nothing else until the next gap, and while a stack holds me the finder
camera's pointings change no rate between my gaps. Once the stack has gone quiet I
take my own corrections again, but only in its gaps: what a finder pointing would have
changed waits for the next gap. I stay so until the stack hands me a tracking error
again, or lets me go.

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
   drift is above 3.0″. **When** on a rig built the same way, with the same task
   running, it plays 40 subs with feedback, **Then** every true drift from the 12th on
   is at most 0.15″ (oracle: the simulator's truth; the noise is the research note's
   0.3′ to 0.5′ a finder solve).
3. **Given** the tracker rig with no rate error, **When** a test awaits one `gap`, hands
   the axis displacement (0.0, 1.9 counts) as a tracking error at the time now, and
   awaits a second `gap`, **Then** a committed test asserts that the second gap answers
   a `Stepped` of exactly (0.0, 0.0), that the tracking offset is exactly `[0.0, 0.0]`
   (a first error sets no rate), and that the truth error just after it is within
   0.05″ of the one just before. **When** on a fresh tracker rig built the same way the
   error handed over after the first gap is the axis displacement (0.0, 2.1 counts),
   **Then** the second gap's `Stepped` is within 0.05″ of −0.5 times the error handed
   over, the truth error changes across the gap by that `Stepped` within 0.05″, the
   gap takes between 0.5 s and 0.6 s, and every azimuth rate the rate log holds from
   the gap's call to its answer equals the azimuth feed-forward rate at its time
   within 0.001″/s. **And** on a third fresh rig, for the axis displacement
   (3.0 counts, 0.0), the same holds with the two axes exchanged: the altitude rates
   are the ones that stay at the altitude feed-forward rate (oracle: the simulator's
   truth and its transforms; the deadband is two counts, 1.6″ here).
4. **Given** the tracker rig with no rate error, **When** a test awaits a `gap`, hands
   the tracking error (12.0, −16.0) and awaits a second, **Then** a committed test
   asserts a `Stepped` within 0.1″ of (−6.0, 8.0) and a tracking offset of
   `[0.0, 0.0]`: a first error sets no rate. **When** it then waits 6 s, hands (6.0,
   −8.0) at the time now, and awaits a third gap, **Then** the `Stepped` is within 0.1″
   of (−3.0, 4.0) and the tracking offset is `[0.0, 0.0]` within 0.002″/s: the second
   error is where the first step left the target, so no drift rate was measured.
   **When** it waits 6 s more, hands (3.0 + 1.3, −4.0) at the time now and awaits a fourth, **Then**
   each axis's tracking offset is within 0.002″/s of −0.3 × d/Δt, where Δt is the time
   between the third and second errors' times as the test handed them, and d is that
   axis's part of a sky offset of (1.3, 0.0), by the differencing of
   `crates/finder/tests/motion/tracking/pi_loop.rs:481` (oracle: the simulator's
   horizon transform, differentiated by the test).
5. **Given** the tracker rig with no rate error, a first gap, the error (0.0, 0.0), a
   second gap, 6 s waited and then the axis displacement (−20.0″, 0.0) handed as an
   error, **When** a third gap is awaited, **Then** a committed test asserts that the
   tracking offset is within 0.002″/s of `[0.0, 0.0]` (a jump of about 3″/s on the
   azimuth axis is over twice the 0.752″/s the offset may hold, so it is not taken for
   a drift rate; the tolerance is for the altitude axis, where the tracker's own
   resolving of the same displacement leaves a few hundredths of an arcsecond, which
   is a drift rate of thousandths) and that the `Stepped` is within 0.2″ of −0.5 times
   the error. **When** after 6 s more the axis displacement (−100.0″, 0.0) is handed
   and a fourth gap awaited, **Then** the `Stepped`'s length is
   within 3% of the length of the axis displacement (30.0″, 0.0): one gap moves an axis
   30″ at most. **When** on another rig the axis displacement (60.0″, 0.0) is handed
   after a first gap and a second gap awaited, **Then** the `Stepped`'s length is within
   0.3″ of the length of the axis displacement (r/2, 0.0), r being the azimuth
   feed-forward rate then in arcseconds a second, about 19.9: a step against the axis's
   motion can only hold the axis still for the half second (oracle: the simulator's
   truth and transforms; the displacements are negative in the first two cases so that
   the step runs with the azimuth axis, which turns forward at M42's place then).
6. **Given** the tracker rig with no rate error and a first gap, **Then** a committed
   test asserts that:
   - `tracker.pointing(30.0, 0.0)` called every 2 s for 20 s changes no rate: every
     rate in the log over those 20 s equals its axis's feed-forward rate within
     0.001″/s, and the tracking offset stays `[0.0, 0.0]`; and `state().samples` holds
     at least 9 samples of 30″;
   - three more gaps with no `steer` between them make `steered_by_subs()` false after
     the third, and a fourth leaves it false and answers exactly (0.0, 0.0): a claim
     that lapsed stays lapsed while no tracking error comes. Then
     `tracker.pointing(30.0, 0.0)`, called twice 2 s apart, changes no rate: every
     rate the log gains from the first call until the next gap's call equals its
     axis's feed-forward rate within 0.001″/s, and the tracking offset stays exactly
     `[0.0, 0.0]` (US3-S11 says what a gap does with those pointings). Then
     `tracker.steer` with (0.0, 0.0) at the time now and one more gap make
     `steered_by_subs()` true again; that gap answers exactly (0.0, 0.0), and the
     tracking offset read after it is the one read just before it, exactly: the error
     that takes the tracker back sets no rate, and the correction held from the two
     pointings is forgotten;
   - on another rig, `release()` after a first gap makes `steered_by_subs()` false at
     once; a `steer` of (12.0, −16.0) after it is dropped, so the next gap answers
     exactly (0.0, 0.0), and that gap makes `steered_by_subs()` true again, as a new
     stack's first gap does;
   - on another rig, `tracker.steer` with a value that is not finite after the first
     gap, and on yet another a `steer` of (12.0, −16.0) made before any gap, each
     leave the next gap answering exactly (0.0, 0.0) with a tracking offset of exactly
     `[0.0, 0.0]`.
7. **Given** the tracker rig with a rate error of 0.01 and ten subs of 5 s played,
   **Then** a committed test asserts of the rate log that between each gap's answer and
   the next gap's call every rate equals its axis's feed-forward rate plus the tracking
   offset read just after that answer, within 0.001″/s; that every gap put at least one
   rate for each axis into the log between its call and its answer; and that after
   `tracker.stop("stopped")`, a `gap` answers (0.0, 0.0) at once and the log gains no
   rate (oracle: 015's own rule that nothing follows a stop,
   `specs/015-tracking/spec.md:229`). **Given** a tracker rig with no rate error, built
   without the wait for its first rates, **When** a test awaits `gap` as the first
   thing after `start` answers, **Then** a committed test asserts that it answers
   exactly (0.0, 0.0), that `steered_by_subs()` is false after it, and that, 0.1 s
   later, the first rate the log holds for each axis equals that axis's feed-forward
   rate at its time within 0.001″/s: a gap that comes before a tracker's first rates
   claims nothing and sends nothing, so no axis is sent a zero before it has started
   (oracle: the rig's rate log and the test's own feed-forward rate).
8. **Given** the console `crate::support::goto::Rig` builds
   (`crates/finder/tests/support/goto.rs:113`) at altitude 50°, azimuth 120°, pointed at
   M42 with `point_eyepiece_at`, synced, and tracking M42 through `POST /api/track` with
   no GoTo, as `crates/finder/tests/motion/tracking/routes.rs:96` does, **When** a test
   waits 2 s after the session first reports `tracking`, so that the tracker has sent
   its first rates, takes the session's tracker (`state.session().tracker()`), awaits
   `gap`, and for 20 s hands the error (0.0, 0.0) at the time now and awaits `gap`
   every 2 s, **Then** a committed test asserts that the newest sync's `time` in the
   session's pointing model advanced by at least 15 s over those 20 s (the model
   keeps one sync of a place and replaces it,
   `crates/finder/src/model.rs:512`), that the tracker's `state().samples` grew, that
   `steered_by_subs()` is true throughout, and that the tracking offset at the end is
   the one read after the first gap within 0.002″/s (oracle: the session's own model;
   the finder camera still feeds it).
9. **Given** the tracker rig with no rate error and a first gap, **When** a test reads
   the console time t, hands the tracking error (12.0, −16.0) with the time t − 3.0,
   awaits a second gap, hands (12.0, −16.0) again with the time t − 1.0, and awaits a
   third gap, **Then** a committed test asserts that the second gap's `Stepped` is
   within 0.1″ of (−6.0, 8.0); that the third's is within 0.1″ of (−3.0, 4.0), not
   (−6.0, 8.0): the second error was measured before the first step, which has since
   halved it; and that the tracking offset is exactly `[0.0, 0.0]` after the second gap
   and within 0.002″/s of it after the third (worked example: (12, −16) + (−6, 8) =
   (6, −8), and half of that is taken out; no step lies between the two errors' times,
   so no drift rate was measured but the turn of the sky's axes in 2 s).
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
11. **Given** the tracker rig with no rate error, a first `gap`, and three more gaps,
    each after 6 s of waiting and with no `steer` between them, so that
    `steered_by_subs()` is false after the fourth, **When** a test reads the tracking
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
      azimuth and −7.2″ of altitude, and both axes turn forward, so the step of about
      +23.9″ and +4.3″ runs with both and is under the 30″ an axis one gap may move);
    - each axis's tracking offset after the gap is within 0.002″/s of −0.6 × d/60, d
      being that axis's part of a sky offset of (30.0, 0.0) by the differencing of
      `crates/finder/tests/motion/tracking/pi_loop.rs:481` (worked example: 015's
      integral adds 0.6 of a pointing's axis error over 60 s for every pointing that
      has an earlier one, `crates/finder/src/tracking.rs:305`; the first pointing after
      the claim has none, so only the second adds, about +0.40″/s in azimuth and
      +0.07″/s in altitude);
    - `steered_by_subs()` is still false after that gap (oracle: the simulator's truth
      and its horizon transform, differentiated by the test, and 015's constants).

    **Given** the tracker rig with a rate error of 0.01 and the second task of US3-S2,
    which calls `tracker.pointing` every 2 s with the truth error plus Gaussian noise
    of σ 15″ on each of east and north, **When** a test awaits a first gap, three more
    6 s apart with no `steer`, and then ten more 6 s apart, **Then** a committed test
    asserts that `steered_by_subs()` is false after the fourth gap and after every
    later one; that from the answer of each of the ten last gaps to the next gap's
    call, and after the last of them for the 6 s that follow its answer, every rate in
    the log equals its axis's feed-forward rate plus the tracking offset read just
    after that answer within 0.001″/s; and that the tracking offset read after the last
    of the ten differs from the one read after the fourth gap: once the subs have
    lapsed, the finder camera's corrections reach the mount in the gaps, and only there
    (oracle: the rig's rate log and the test's own feed-forward rate; with σ 15″ on
    each coordinate a pointing falls inside 015's 10″ deadband at most about one time
    in five, 1 − e^(−100/450), so the chance that the 29 or so pointings with an
    earlier one in those ten gaps all leave the offset alone is under 10⁻¹⁸, whatever
    the seed).

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
   `exposure_s` 5.0 and `limited` false; and that after `accepted` with the worked
   frame (`start` 1768528800.0, `exposure_s` 5.0) and the worked solve, `view()`'s
   `source` is `none`, its `subs` 0 and its `last` absent. **Given** a scripted
   steering whose tracker has no target, **Then** `accepted` with the same frame and
   solve after a `before_exposure` puts no `steer` in the log and leaves `subs` 0.
3. **Given** a scripted steering after one `before_exposure`, **Then** a committed test
   asserts that `accepted` with a frame whose `start` is 1768528700.0, a hundred seconds
   before that call, puts no `steer` in the log and leaves `subs` 0: no exposure of this
   stack holds its middle; that dropping the `Hold` that `Steering::hold` answered puts
   one `release` in the log; and that `accepted` with the worked frame after it puts no
   `steer` in the log.
4. **Given** a scripted steering with `set_set_point(Some((718.0, 361.0)))`, **Then** a
   committed test asserts that the `steer` after a `before_exposure` and `accepted` with
   the worked frame and solve carries US2-S2's east and north within 0.001″; and that
   with no set point, a `before_exposure` and a frame 200 × 200 whose `roi` is
   (400, 200, 400, 400) at binning 2, solved by a solve 200 × 200 with the worked
   solve's centre and twice its matrix, it carries −33.2577 ± 0.001 east and
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
6. **Given** the console of US3-S8 before Track is turned on, **Then** a committed test
   asserts that `session.steered()` is `None`; that once the session reports tracking it
   answers a tracker whose `target()` is M42's catalogue place, (83.81867, −5.38967),
   exactly; and that after `POST /api/track` with `{"on":false}` it is `None` again
   (oracle: the session's own Track state).

---

### User Story 5 - The exposure cap (Priority: P2)

As the observer, I choose how many pixels the field may turn at the corner of a sub
during one exposure. A stack then starts with no exposure longer than the tracker
allows for that, halving the exposure I set as often as needed, keeps that exposure
while it runs, and the readout says when my exposure was cut and when the limit has
since fallen under it.

**Why this priority**: Field rotation is the one limit no correction removes. It is P2
because short exposures chosen by hand already stay under it.

**Independent Test**: `crates/finder/tests/console/steering_cap.rs`, the module
`steering_cap` of `console`, and, for US5-S6, `crates/finder/tests/motion/steering_cap.rs`,
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
   - the turn asked about equals the test's own `2.0 / 618f64.hypot(411.0)` radians
     within 10⁻¹² (worked example: the corner is hypot(618, 411) = 742.189 px from the
     centre, and the smear is 2 px);
   - a wanted exposure of 60 s answers 30.0 with `limited` true, and 20 s answers 20.0
     with `limited` false;
   - at binning 2 the turn is `2.0 / 309f64.hypot(205.5)`; for the region
     (400, 200, 400, 400) at binning 1 it is `2.0 / 218f64.hypot(211.0)`; with the set
     point (718.0, 361.0) on the whole sensor it is `2.0 / 718f64.hypot(461.0)`; each
     within 10⁻¹² (worked example: the farthest corners are 371.095 px, 303.389 px and
     853.256 px from the set point; the figures here are rounded and are not what the
     test compares with);
   - with a longest exposure of 6.0 s, a wanted 60 s answers 3.75; with that and a
     shortest exposure of 10 s it answers 15.0, `limited` true; with no longest
     exposure it answers 60.0, `limited` false.
2. **Given** the same steering and a first `before_exposure` that answered 30.0 for a
   wanted 60 s, **When** the tracker's longest exposure becomes 25.0 s and a test awaits
   `before_exposure` again without a `release`, for a wanted 60 s and then for a wanted
   10 s, **Then** a committed test asserts that both answer 30.0 with `limited` true:
   the exposure, and whether it was cut when it was decided, are held until `release`,
   whatever is asked for meanwhile; that `view()` has `exposure_s` 30.0, `limited`
   true, `longest_exposure_s` 25.0 and `over_limit` true; and that after `release` a
   `before_exposure` for a wanted 60 s answers 15.0 with `limited` true, and `view()`
   has `over_limit` false.
3. **Given** the same after `set_smear_px(1.0)`, **Then** a committed test asserts the
   turn asked about equals `1.0 / 618f64.hypot(411.0)` within 10⁻¹², and that
   `set_smear_px` refuses 0.05, 20.5 and a value that is not finite with
   `smear_px must be a number from 0.1 to 20`.
4. **Given** a scripted steering whose tracker answers a longest exposure of 37.15 s,
   after a first `before_exposure` with the worked ask and a wanted exposure of 60 s,
   **Then** a committed test asserts `view()` has `smear_px` 2.0, `on` true,
   `exposure_s` 30.0, `limited` true, `over_limit` false and `longest_exposure_s` 37.15;
   and on another, whose tracker names no longest exposure, after the same call,
   `longest_exposure_s` is the string `no limit`, `exposure_s` 60.0 and `limited`
   false. **Given** a scripted steering whose tracker answers 37.15 s, with
   `set_on(false)`, **Then** `before_exposure` with the worked ask and a wanted 60 s
   puts no `gap` in the log and answers 30.0 with `limited` true, `accepted` with the
   worked frame and solve puts no `steer` in the log, and `view()`'s `source` is
   `tracker`.
5. **Given** 032's solving rig (032 spec.md § User Scenarios), **Then** a committed test
   asserts that:
   - `PUT /api/imaging/steering` with `{"smear_px":1.5}` answers 200 with the imaging
     view, whose `steering.smear_px` is 1.5 and `steering.on` true, and
     `GET /api/session`'s `imaging.steering` says the same;
   - `{"on":false}` answers 200 with `steering.on` false and `smear_px` still 1.5, and
     `{}` answers 200 and changes nothing;
   - `{"smear_px":0}`, `{"smear_px":"2"}`, `{"on":1}` and `{"smear":2}` each answer 400
     with a JSON `error`, the first exactly
     `smear_px must be a number from 0.1 to 20`, and change nothing;
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
   19.12 µm are 23.63232 × 15.71664 mm; at 14.96°/h the answer is 37.15 s).
   **Given** a tracker rig whose target is due east at an instant the test chooses,
   t* = 1768528830.0, 30 s after the rig's clock starts: with Dec fixed at +18.75° the
   test steps RA through 0°, 1°, … 359°, takes the first two neighbours whose azimuths
   at t* by `sky::alt_az` for 40° N 75° W lie either side of 90° with both altitudes
   above 0°, and bisects RA between them 60 times, so that the azimuth at t* is 90°
   within 10⁻⁹ degrees (the altitude is then near 30°); the rig's target is a clone of
   M42's `Target` with that `ra_deg` and `dec_deg`, and its mount's zero points put
   its axes on that place's apparent direction at the start, as the tracker rig does
   for M42; **When** the test sleeps until the rig's clock reads t* to within 2 ms
   (tokio's timer counts in milliseconds) and calls `longest_exposure_s(0.0027)` with
   no await after the sleep, **Then** a committed test asserts it is `None`; and that
   after 10 s more it is `Some` of more than 3600 s (oracle: 015's own function and
   its 10⁻⁹ rad/s rule. Worked example: due east at this place the azimuth moves 9.7″
   a second, and the field rotation passes 10⁻⁹ rad/s 3.2″ of azimuth from due east,
   a third of a second either side of t*, so the instant matters and 2 ms is 0.02″;
   10 s later the azimuth is 97″ past east, the rotation 3 × 10⁻⁸ rad/s and the answer
   about 89,000 s).

---

### User Story 6 - Not steady: pause, and drop what was taken across it (Priority: P2)

As the stack, I start no exposure while the tracker says it is not steady: during a
backlash take-up, a first reversal that is still being measured, or the keyhole pass.
A sub whose exposure crossed such a time is dropped before it is solved, with that
reason. While it is not steady the tracker corrects itself as 015 has it, even after
the subs have lapsed, so that a first reversal's measurement still closes.

**Why this priority**: Altitude reverses at transit, where a target is highest and a
run will sit. It is P2 because it is rare within a night.

**Independent Test**: `crates/finder/tests/motion/steering_steady.rs`, the module
`steering_steady` of `motion`, on the backlash rig; and
`crates/finder/tests/console/steering_steady.rs`, the module `steering_steady` of
`console`, with the scripted tracker. The `test` gate's
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
   `rig.tracker.steady()` and `rig.tracker.steered_by_subs()`; and the asker, which
   looks every 10 ms for the reader's first false reading of `steady()`, then awaits
   `rig.tracker.gap()` once and notes the console time of its answer, **Then** a
   committed test asserts, with F and L the times of the reader's first and last
   false readings, that:
   - `steady()` read false at least once, and every reading from F to L is false;
   - the altitude rate of magnitude 0.5°/s in the rate log, and the zero rate that ends
     it, both lie between F − 0.05 s and L + 0.05 s (oracle: the rig's rate log, as
     `crates/finder/tests/support/tracker.rs:285` reads it);
   - `steady_between(F - 20.0, F - 1.0)` and `steady_between(L + 1.0, L + 20.0)` are
     true, and `steady_between(F - 5.0, F + 0.1)`, `steady_between(L - 0.1, L + 5.0)`
     and `steady_between(F - 5.0, L + 5.0)` are false, all asked once the three
     futures have ended;
   - the reader's `steered_by_subs()` is true at its reading 1 s before F and false at
     L;
   - the asker's `gap` answered after L.
2. **Given** `crate::support::tracker::Rig` for M42 from 120 s before its transit on a
   fresh session, so that no backlash is measured, **When** a test joins
   `rig.follow(transit + 120.0, 60.0)` with a reader as in US6-S1, **Then** a committed
   test asserts, with Z the time of the altitude axis's first zero rate in the rate
   log, that every reading of `steady()` from 1 s after the rig's start to Z − 0.1 s
   is true, that the reading nearest Z + 5 s is false, and that
   `steady_between(Z + 0.1, Z + 10.0)`, asked at the end, is false. **Given** the
   backlash rig as `Rig::new` leaves it, with the console time s read as soon as it
   answers and no wait, **Then** the rate log holds no rate and `steady()` is false; a
   `gap` awaited then answers exactly (0.0, 0.0) and leaves `steered_by_subs()` false;
   0.1 s later the log holds a rate for each axis and `steady()` is true;
   `steady_between(s - 20.0, s - 10.0)` is false: it was not tracking then; and after
   `rig.follow(s + 10.0, 60.0)` and `stop("stopped")`, with e the console time read
   just before the stop, `steady()` is false, `steady_between(e - 3.0, e + 0.1)` and
   `steady_between(e + 1.0, e + 2.0)` are false, and `steady_between(e - 3.0, e - 1.0)`
   is still true (oracle: the rig's rate log and clock).
3. **Given** a scripted steering whose tracker says it is not steady and whose `gap`
   waits 0 s, **When** a test awaits `before_exposure` with the worked ask in a task,
   waits 10 s, sets the tracker steady and waits 1 s, **Then** a committed test asserts
   that the call had not answered after the 10 s, with `view()`'s `steady` false,
   `waiting` true, `Steering::waiting()` true and no `gap` in the log; and that it
   answered within 0.5 s of the change, after exactly one `gap`, with `steady` true,
   `waiting` false and `Steering::waiting()` false. **Given** a scripted steering whose
   tracker's `gap` waits 1 s, **When** a test awaits `before_exposure` in a task, sets
   the tracker not steady 0.5 s later and steady again 5 s after that, **Then** the
   call has not answered 4 s after it was made, and it answers between 6.4 s and
   7.0 s after it was made, with exactly two `gap`s in the log: the first gap ended
   on a tracker that was not steady, so it does not count. **Given** a scripted
   steering whose tracker is steady, after one `before_exposure`, and a `Steering`
   whose source answers no tracker, after one, **Then** each `view()` has `steady`
   true, `waiting` false and `dropped_not_steady` 0. **Given** a scripted steering
   whose tracker is not steady, a `Hold` taken from it and a `before_exposure` awaited
   in a task for 2 s, **When** the test aborts that task and drops the `Hold`, **Then**
   `Steering::waiting()` is false and `view()` has `waiting` false and `steady` true: a
   wait abandoned with its stack does not go on showing.
4. **Given** a scripted steering after a `before_exposure` with the worked ask, whose
   tracker answers `steady_between` false, **Then** a committed test asserts that
   `check` with the
   worked frame (`start` 1768528800.0, `exposure_s` 5.0) answers
   `Verdict::Drop("not_steady")`, and that `view()`'s `dropped_not_steady` is 1; that
   with `steady_between` true it answers `Verdict::Keep` and the count stays 1; that the
   tracker's log shows the window it was asked about as (1768528800.0, 1768528805.0);
   that after the test has made the source answer no tracker and dropped its last
   `Arc` of the scripted one, `check` with the same frame answers
   `Drop("not_steady")`: a tracker that has gone was not steady; and that on a
   `Steering` whose source answers no tracker, `check` after a `before_exposure` answers
   `Keep`.
5. **Given** `crate::support::tracker::Rig` for M42 from 600 s before its transit on a
   fresh session, as `crates/finder/tests/support/tracker.rs:326` builds one, so that
   no backlash is measured; a first `gap` awaited once the rate log holds a rate for
   each axis (read every 10 ms, for at most 5 s, as the tracker rig waits); and three
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
`POST /api/track`; then it waits until the session's `track.state` is `tracking` and
the imaging view's `state` is `ready`. **Waiting until** a condition means reading
`GET /api/imaging`, or `GET /api/session` for the session's parts, every 100 ms until
it holds, for at most 300 s of virtual time, and failing otherwise. **A sub's drift**
is the separation between `sim.imaging_truth_at` at the sub's `start` and at `start`
plus its exposure, both from 031's `frame` part of the imaging view, which the test
reads every 100 ms and keeps for every imaging frame id it sees `saved`; subs are
numbered in the order their ids rise. **The stack's counts** are the `stack` part of
the imaging view (035); **Σ dropped** is the sum of the twelve counts of its `dropped`.

**Acceptance Scenarios**:

1. **Given** the stacking rig with `set_rate_error(0.01)` on its mount and
   `PUT /api/imaging/settings` with `{"exposure_ms":5000}`, **When** a test starts a
   stack with `POST /api/imaging/stack` and waits until `steering.subs` is at least 18,
   **Then** a committed test asserts that `steering.source` is `subs`; that every
   sub's drift from the 14th sub on is at most 0.15″; that `stack.dropped.not_steady`
   is 0; and that after `DELETE /api/imaging/stack`, `steering.source` is `tracker` and
   `steering.subs` 0. **When** on a rig built the same way the test calls
   `sim.stop_frames()` (`crates/simulator/src/controls.rs:106`) as soon as the session
   reports `tracking`, so that no finder pointing reaches the tracker and its own
   corrections change no rate, then sends `PUT /api/imaging/steering` with
   `{"on":false}`, starts a stack and waits until 12 subs are saved, **Then**
   `steering.subs` stays 0, `steering.source` is `tracker`, and at least 8 of the 12
   subs drift more than 0.5″ (oracle: the simulator's truth; worked example: with
   nothing correcting it, a mount 1% fast drifts 0.749″ in a 5 s sub. The finder
   camera's frames are stopped because 015's own loop would otherwise start to
   correct once the error passed its 10″ deadband, about 67 s after Track began, and
   the bound would then rest on how long the rig's waits took).
2. **Given** the stacking rig and `{"exposure_ms":60000}`, **When** a stack is started
   and its first sub is saved, **Then** a committed test asserts that sub's
   `exposure_ms` is 30000.0; that `settings.exposure_ms` is 30000.0 while the stack
   runs; that `steering.limited` is true, `steering.exposure_s` 30.0,
   `steering.over_limit` false and `steering.longest_exposure_s` within 2% of the
   figure the test computes as US5-S6's first half does for the console time of its
   read; and that `settings.exposure_ms` is 60000.0 once `DELETE /api/imaging/stack`
   has answered (the stack set the exposure through its own lease and put the
   observer's back through it before letting the camera go).
3. **Given** the stacking rig with `{"exposure_ms":5000}`, a stack running and
   `stack.stacked` at least 2, **When** a test waits until an imaging frame is
   `exposing`, sends `POST /api/track` with `{"on":false}` and then `{"on":true}`,
   waits until `stack.taken` has risen by 3 more and stops the stack, **Then** a
   committed test asserts `stack.dropped.not_steady` is at least 1,
   `steering.dropped_not_steady` equals it, and `stack.taken` equals `stack.stacked`
   plus Σ dropped.
4. **Given** 032's solving rig as it stands, with no target and Track off, and
   `{"exposure_ms":2000}`, **When** a test starts a stack, waits until `stack.taken` is
   at least 3 and stops it, **Then** a committed test asserts that `steering.source` is
   `none` and `steering.subs` 0 throughout, that every saved sub's `exposure_ms` is
   2000.0, and that `stack.dropped.not_steady` is 0: with no tracker the stack is
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
   the link is lost within 3 s by 031 US1-S6, well inside the gap's 10 s. A stack
   that cannot shorten its exposure takes no sub at the observer's).
6. **Given** the text of `crates/finder/src/imaging/stack/service.rs`, 035's file as
   this story leaves it, read by the test from `CARGO_MANIFEST_DIR`, **Then** a
   committed test asserts it holds none of `ephemeris`, `alt_az`, `parallactic`,
   `counts_per_revolution`, `AxisInfo` and `MountClient` (oracle: the plan document's
   rule 1, read as US2-S6 reads it of `steering.rs`: the stack reaches the tracker only
   through `Steering`, and the calls this story adds to it say nothing of axes).

---

### User Story 8 - The readout on the Imaging page and in the session (Priority: P3)

As the observer, I see on the Imaging page what steers the tracker, how large the
tracking error is, how far the target drifts during a sub, in arcseconds and in pixels,
whether my exposure was cut, and whether the stack is waiting for the tracker. I set
the corner smear and the switch there.

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
   after the second sub `drift_arcsec` is 0.76923 ± 10⁻⁴ and `error_rms_arcsec`
   0.70711 ± 10⁻⁴; and after the third `drift_arcsec` is 0.58583 ± 10⁻⁴, `drift_px`
   0.27122 ± 10⁻⁴, `error_rms_arcsec` 0.90370 ± 10⁻⁴ and `error_rms_px`
   0.41838 ± 10⁻⁴ (worked example: the drifts are 1.0″ × 5/6.5 and 0.4″ × 5/6.5, the
   second after the step of (−0.5, 0.0) is taken out; the readout is their root mean
   square, and the errors'; the scale is 2.16″ a pixel). After one sub, none of the
   four keys is present.
2. **Given** the same steering after 12 subs 6.5 s apart whose errors are (1.0, 0.0) for
   the first two and (0.0, 0.0) for the last ten, **Then** a committed test asserts
   `error_rms_arcsec` is 0.0: the window is the last ten. **And** after `release` and a
   new `before_exposure`, none of the four keys is present.
3. **Given** `SteeringBlock` rendered with a view whose `steering` is
   `{"smear_px":2,"on":true,"source":"subs","steady":true,"waiting":false,"subs":12,"dropped_not_steady":0,"exposure_s":5,"limited":false,"over_limit":false,"longest_exposure_s":9.29,"error_rms_arcsec":0.9037,"error_rms_px":0.41838,"drift_arcsec":0.58583,"drift_px":0.27122}`,
   **Then** a committed test asserts one region named `Tracking from the subs` whose
   status reads exactly `Steered by the subs · 12 subs`, and terms reading
   `Tracking error (RMS, last 10 subs)` with `0.9″ · 0.4 px`, `Drift per sub` with
   `0.6″ · 0.3 px`, `Exposure` with `5.0 s` and `Tracker's limit` with `9.3 s`; and no
   alert.
4. **Given** the same with `limited` true, `exposure_s` 30 and `longest_exposure_s`
   37.2, **Then** `Exposure` reads exactly `30.0 s, cut to fit the tracker's limit`
   and `Tracker's limit` reads exactly `37.2 s` (37.2 and not the worked limit of
   37.15, which a binary float holds a hair under and JavaScript's `toFixed(1)` prints
   as `37.1`: the test is of the words, not of rounding a tie). With
   `longest_exposure_s` `"no limit"` the
   limit reads `none`. With `over_limit` true and `longest_exposure_s` 25.1 an alert
   reads exactly
   `The exposure is longer than the tracker's limit of 25.1 s. Stop the stack and start it again to shorten it.`
   With `source` `tracker` the status reads `Steered by the tracker's own corrections`;
   with `source` `none`, `Track is off: nothing steers`; with `steady` false and
   `waiting` true it reads `Exposures paused: the tracker is not steady`; with
   `dropped_not_steady` 2 a term `Dropped while not steady` reads `2`. With no
   `steering` in the view the region shows the form and no status.
5. **Given** `SteeringBlock` with the view of US8-S3, **When** a test sets the field
   `Corner smear (px)` to 1.5, unticks `Steer the tracker from the subs` and clicks
   `Save tracking settings`, with `fetch` answering 200 and a view, **Then** a
   committed test asserts `fetch` was called with `/api/imaging/steering` and
   `{"method":"PUT","headers":{"Content-Type":"application/json"},"body":"{\"on\":false,\"smear_px\":1.5}"}`,
   then `onChange`; and that a 400 answer with
   `{"error":"smear_px must be a number from 0.1 to 20"}` shows an alert with exactly
   that text and calls no `onChange`.
6. **Given** `ImagingPage` rendered as 031's own test renders it, with a view that holds
   the `steering` of US8-S3, **Then** a committed test asserts the region
   `Tracking from the subs` is present and comes after 035's stack block in document
   order. **Given** 035's `StackBlock` rendered with a view whose `stack` is stacking
   with `dropped` holding `not_steady` 2 and every other count 0, **Then** its line
   reads exactly `Dropped 2: 2 not steady`; and with no `not_steady` key in `dropped`
   and `behind` 4 it reads `Dropped 4: 4 behind`, as before.

---

### Edge Cases

- **No Track.** With Track off, or armed and waiting for a GoTo, there is no tracker to
  steer: the stack is neither capped nor paused and hands over nothing (US7-S4). Subs
  taken during a slew fail 035's own gates.
- **Tracking ends during a stack.** The sub in flight is dropped as `not_steady`; later
  subs find no tracker. Stopping the stack is 035's and 037's.
- **Stop.** Stop ends tracking as before. A gap that is waiting answers at once with no
  step, and no rate follows the Stop (US3-S7).
- **Subs that are not accepted.** Each is a gap with no tracking error. After three
  gaps in a row with none, the tracker's own corrections steer again, with the tracking
  offset as it stood, and go on steering through every later gap until a sub is
  accepted, applied only in the gaps: a finder pointing between two gaps is held for
  the next (US3-S11). The gap after that sub's tracking error takes the tracker back,
  forgets what was held and steps without setting a rate: no drift rate is measured
  across the break (US3-S6).
- **The switch off.** No stack claims the tracker, so its own corrections run as 015
  has them, during exposures too: the observer's declared exception (owner decision 8).
- **A gap before a tracker's first rates.** It answers at once, moves nothing and
  claims nothing (US3-S7); from US6 the stack does not ask for one, because such a
  tracker is not steady yet.
- **A tracking error that arrives late.** It is judged against the steps and rate
  changes made since its mid-exposure (US3-S9, US3-S10). Of two that arrive between two
  gaps, the newer is used.
- **The worker falls behind.** 035 drops the oldest waiting subs as `behind`; fewer
  tracking errors arrive, and after three gaps with none the tracker's own corrections
  steer, in the gaps, until the next accepted sub.
- **A wrong imaging solve.** 035 does not accept a sub whose solve differs from the
  reference sub's in scale or lies off the field, so it steers nothing. For one that
  passes, one gap moves an axis 30″ at most, and a jump between two subs faster than
  twice the largest tracking offset changes no rate (US3-S5).
- **The first reversal of an axis on this mount.** Its backlash is still to be measured
  and the measurement takes minutes; the tracker is not steady for all of it, exposures
  pause, and the finder camera steers as 015 has it, even when the subs had lapsed
  before it (owner decision 17, US6-S5).
- **A correction against an axis's motion.** An axis is never reversed by a step or a
  rate: a step against its motion can at most hold the axis still for the half second,
  and what is left waits for the next gap (FR-008). Near a reversal, where an axis
  crawls, such an error leaves slowly.
- **A long gap.** The feed-forward rates still go out once a second. Nothing times a
  gap out: a stack that is waiting for a tracker in the keyhole waits until it leaves,
  and stopping the stack ends the wait.
- **A sub of a region, or binned.** The set point is a place on the sensor; it need not
  lie inside the frame (US4-S4).
- **The limit falls under the exposure in use.** The stack keeps its exposure and the
  readout says `over_limit`; a new stack starts shorter (US5-S2).
- **The cap near the zenith.** The limit shrinks toward the keyhole. The exposure stops
  halving at the camera's shortest and is then both `limited` and `over_limit`.
- **The observer edits the exposure during a stack.** The edit is refused: while the
  stack holds the camera's lease, `PUT /api/imaging/settings` answers 409 with the
  stack's name (035 plan.md § The camera's lease; 035 US5-S8). So the exposure a stack
  holds cannot change under it. The observer stops the stack, edits and starts again;
  the setting they then see is their own, put back when the stack ended.
- **The exposure cannot be shortened.** If the camera refuses the stack's change of the
  exposure before its first sub, which it does only when it is no longer ready, the
  stack ends with the reason `camera` and the refusal's words, and no sub is taken at
  the observer's longer exposure (US7-S5).
- **The observer's exposure cannot be put back.** If the camera is no longer ready when
  a stack ends, the change back is refused and the setting stays at the exposure the
  stack used; the stack's own ending is not changed by it. The page shows the setting.
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
  telescope is pushed by hand, a mount is connected or its optics are set. 030's
  `imaging_truth_at` and the imaging camera's own rotation SHALL take the direction for
  an instant between two remembered samples by a straight line between them, azimuth
  the short way round, and otherwise the direction now, as before (owner approval 5).
  `directions_at` SHALL not change.
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
  the field (none by default) and `steady` with `steady_between` (true by default). The
  file SHALL hold none of `ephemeris`, `alt_az`, `parallactic`,
  `counts_per_revolution`, `AxisInfo` and `MountClient`: it computes no altitude,
  azimuth, parallactic angle or field rotation and reads no axis count, as the plan
  document's rule 1 asks of every file under `crates/finder/src/imaging/` (FR-023
  holds 035's `stack/service.rs` to the same).
- **FR-006**: `GLOSSARY.md` SHALL define **Tracking error** and **Set point** (owner
  approval 4).

**The tracker** (US3):

- **FR-007**: A tracker SHALL be steered by the subs from a stack's first `gap` that
  comes while it is tracking and after its first rates. While it is, a live pointing
  from the finder camera SHALL change no rate
  and no tracking offset; it SHALL still be watched for the cable's wind, taken as the
  keyhole's resume pointing, kept among the 60 s of samples and given to the backlash
  measure. The live loop's syncs SHALL not change (owner approvals 1 and 2).
- **FR-008**: At a gap that follows a new tracking error, the tracker SHALL resolve it
  into its two axes as it resolves a finder error, add to each axis's error the steps
  its gaps made after the error's time, and for each axis whose sum is at least the
  larger of two of that axis's counts and 1″, move that axis by −0.5 of the sum, at
  most 30″, as one rate pulse of 0.5 s added to the axis's rate, never commanding a
  rate against the axis's direction.
- **FR-009**: At the same gap, for each axis, the tracker SHALL add to the tracking
  offset −0.3 of the drift rate: this error less the last one applied, less the steps
  made between their two times, plus what every change of the tracking offset made
  after the earlier time would have moved the axis had it been in force from that time
  to the later one, all over the time between the two errors. It SHALL change no offset
  for the first error after a claim, nor when that drift rate is more than twice the
  offset's limit, and SHALL keep the offset within 015's limit. It SHALL remember what
  its last 16 gaps did.
- **FR-010**: At every gap while steered, and at every gap that applies a held
  correction (FR-011), the tracker SHALL send each axis its rate and answer only when
  both are acknowledged. Between a gap's answer and the next gap, while it stays
  steered or its claim stays lapsed, each once-a-second rate SHALL be the axis's
  feed-forward rate plus the tracking offset as that gap left it, or zero where 015 sends zero in
  place of a rate against the axis's direction; a take-up and the keyhole pass send
  what 015 has them send. Every rate SHALL be sent by the task that sends the
  once-a-second rates, a gap SHALL be served between two of its turns, and none SHALL
  be sent after tracking ends. A gap that comes before a tracker's first rates SHALL
  answer no step, send nothing and claim nothing.
- **FR-011**: The tracker SHALL stop being steered by the subs on `release`, at the
  third gap in a row with no new tracking error, and when tracking ends, keeping its
  tracking offset; after a `release` or the end of tracking the finder camera's
  pointings correct it again as in 015. After such a third gap the claim has lapsed:
  the tracker SHALL stay so, whatever gaps follow, until a tracking error is handed
  over, and while it stays so a live pointing SHALL change no rate when it arrives.
  The step that 015's proportional term would make for it, −0.6 of its axis error, and
  the change that 015's integral would make to the tracking offset SHALL be held
  instead, a newer pointing's step replacing an older one's and the offset's changes
  adding up, and the next gap SHALL apply them: the offset within 015's limit, the
  step as one 0.5 s pulse of at most 30″ an axis added to the axis's rate, never
  commanding a rate against the axis's direction. The gap after a tracking error is
  handed over SHALL take the tracker back, forget what was held and treat that error
  as the first after a claim; `release` and the end of tracking SHALL also forget what
  was held. A tracking error that is not finite, that is handed over before
  a stack's first gap or after a `release`, or that is no later than the last applied
  SHALL be ignored; of two handed over between two gaps the later SHALL be used.
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
  otherwise. The set point SHALL be the sensor's centre unless `set_set_point` has
  named another.
- **FR-015**: `Steering::release`, and the drop of the `Hold` a stack keeps, SHALL
  release the tracker and forget the exposures remembered.
- **FR-016**: The imaging view, and the session's `imaging` object, SHALL carry
  `steering` from the first use of the steering in a run of the console, and not
  before: `source` (`subs`, `tracker` or `none`), `subs` and `last` (owner approval 3).
  `Session::steered` SHALL answer the running tracker, and `Imaging::steering` the
  console's steering.

**The exposure cap** (US5):

- **FR-017**: The tracker's `longest_exposure_s` SHALL answer the turn it is asked
  about divided by the field rotation at the target now, and none when that rotation is
  under 10⁻⁹ rad/s, in agreement with 015's `longest_exposure`.
- **FR-018**: `before_exposure` SHALL ask the tracker about a turn of the smear setting
  divided by the distance, in delivered pixels, from the set point to the farthest
  corner of the frame to be taken. The first call after a `release`, or the first of
  all, SHALL answer the wanted exposure halved until it is no longer than the answer or
  one more halving would pass the camera's shortest, with `limited` true when that is
  shorter than the exposure it was asked for; every later call SHALL answer that same
  exposure and that same `limited`, whatever exposure it is asked for, until the next
  `release`. The view SHALL carry `smear_px`, `on`,
  `exposure_s`, `limited`, `longest_exposure_s`, a number or `no limit`, and
  `over_limit`, true while the limit is under the exposure held.
- **FR-019**: `PUT /api/imaging/steering` SHALL accept, under the same-origin rule, a
  JSON object with either or both of `on` (a boolean) and `smear_px` (a number from 0.1
  to 20), and answer 200 with the imaging view; 400 for any other JSON body and for a
  body that is not JSON, with `smear_px must be a number from 0.1 to 20` for a number
  out of range; 415 for a `Content-Type` other than `application/json`; 403 for a
  foreign origin; and 409 with
  `this console takes no imaging frames` without imaging. Both settings are run-only:
  2.0 and true at every start. With `on` false the steering SHALL call neither `gap`
  nor `steer`, and SHALL still cap and, from US6, still pause and drop (owner approval
  3).
- **FR-020**: `crates/finder/README.md` SHALL document the route in its `## API` table,
  and `crates/finder/src/lib.rs` SHALL list it (owner approval 8).

**Not steady** (US6):

- **FR-021**: The tracker's `steady` SHALL be false from its start until its first rates
  are sent; from the start of a take-up or of the keyhole pass until the rates that
  follow it are sent; while an axis's backlash is being measured; and after tracking
  ends (owner decision 17). `steady_between` SHALL be false for a window that touches
  any such time among the last 64, or that begins before the tracker was started. A
  tracker that stops being steady SHALL stop being steered by the subs and forget any
  tracking error not yet applied and any correction held for a gap; a gap served while
  the tracker is not steady SHALL answer no step and claim nothing, and one asked for
  during a take-up or the keyhole pass SHALL be served once that is over. While the
  tracker is not steady a live pointing SHALL correct it as in 015, even when its
  claim has lapsed.
- **FR-022**: `before_exposure` SHALL not answer, and SHALL call no `gap`, while the
  running tracker is not steady, with `steady` false and `waiting` true in the view;
  it SHALL answer only after a `gap` at whose end the tracker is steady, and the view
  SHALL then have `steady` true and `waiting` false, as it has before any wait.
  `Steering::check` SHALL answer `Drop("not_steady")` for a sub when the tracker that
  `before_exposure` found before that sub's exposure no longer exists, or says it was
  not steady for the whole of the exposure; and `Keep` otherwise, a sub taken with no
  tracker among them. The view SHALL count the drops in `dropped_not_steady`.

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
  `crates/finder/src/imaging/stack/service.rs` SHALL hold none of the six names FR-005
  lists.
- **FR-024**: Giving the console its imaging solver SHALL give its steering the
  session's running tracker as its source, in the binary and under `--simulate` alike.
- **FR-025**: `crates/finder/README.md` SHALL gain a section on tracking from the subs:
  what steers when, the gap, the cap, the pause, and the constants (owner approval 8).

**The readout** (US8):

- **FR-026**: The view SHALL carry, once two tracking errors have been handed over
  since the tracker was last released, `error_rms_arcsec` and `drift_arcsec`, each the
  root mean square over the last ten tracking errors, and the same in delivered pixels
  as `error_rms_px` and `drift_px`, by the latest solve's scale. A sub's drift SHALL be
  the change in tracking error since the sub before, less the steps answered by the
  gaps between their mid-exposures, times that sub's exposure over the time between
  the mid-exposures.
- **FR-027**: The Imaging page SHALL show a region `Tracking from the subs` after 035's
  stack block, with the words of plan.md § The page: what steers, the readout to 0.1″,
  0.1 px and 0.1 s, the tracker's limit, the pause, the count of dropped subs, and a
  form that sends both settings (owner approval 7).
- **FR-028**: `DESIGN.md` SHALL describe the block, `.impeccable/design.json` SHALL list
  it as a component, and `crates/finder/web/dist` SHALL be the build of its source.
- **FR-029**: 035's `StackBlock` SHALL name subs dropped as `not_steady` in its
  `Dropped` line with the words `not steady`, last, and read a `dropped` without that
  key as before (owner approval 7).

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
  point, the two settings, the exposure a stack holds and the readout.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited. The only edit
  to an existing file under `crates/finder/tests` is one `mod` line for each new
  module, in alphabetical order, in `crates/finder/tests/console/main.rs` and
  `crates/finder/tests/motion/main.rs` (owner approval 9).
- **SC-003**: On a simulated mount that runs 1% fast, with tracking errors arriving a
  sub late, the drift within a 5 s sub is at most 0.15″ once the subs steer (from the
  12th sub in US3-S1, the 14th in US4-S5 and US7-S1), against 0.75″ without.
- **SC-004**: Every test of 015 passes unchanged, and neither
  `crates/finder/src/imaging/steering.rs` nor 035's
  `crates/finder/src/imaging/stack/service.rs` holds alt-az arithmetic (US2-S6,
  US7-S6).
- **SC-005**: While a stack runs with the switch on (owner decision 8) and the tracker
  steady (US6-S5), no rate is sent between two of its gaps other than each axis's
  feed-forward rate plus the tracking offset the last gap left, whether the subs steer
  or have lapsed (US3-S7, US3-S11, US4-S5).

## Assumptions

- **Earlier features are in place.** 015 has landed. 030, 031, 032, 033, 034 and 035 are
  drafts (plan.md § Prerequisites). 031's draft carries the camera's lease of the plan
  document's amendment 1 as its US6, in the names 033 and 035 use; this trio reaches it
  through 035 (plan.md § What 035 must provide). Every `path:line` here was read
  at 49bb210, and read again there after the audit.
- **Out of scope:**
  - an equatorial mount, a tracker that corrects in small steps, meridian flips,
    dithering and guiding with a second camera (the plan document, § Deferred);
  - moving the set point: this spec keeps one and lets 037 or a later spec name it;
  - telling a wrong imaging solve from a right one, and star-shape gates (035);
  - changing a stack's exposure while it runs, and weighting subs by exposure (035);
  - stopping a stack when tracking ends, and the run's states (037);
  - retuning 015's gains for the finder camera, and any change to GoTo;
  - measuring a mount's backlash any other way than 015 does;
  - an MCP tool (owner decision 11);
  - persisting the two settings (owner decision 3).
- **Outside the factory:**
  - GoTo and Track proven on the real mount (the plan document, § Before any flip, 4),
    and its altitude backlash measured, before this spec is used at the telescope;
  - the field checks: drift per sub with the switch on and off, on a bright field;
    drift per sub while subs are being dropped, the finder camera correcting in the
    gaps; a run through the meridian; whether a half-second pulse of one count moves
    the tube;
    how late a tracking error arrives with the real solver at the camera's full
    resolution; and the constants of owner decisions 4 and 5 confirmed or retuned, as
    constants in code;
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
  depends_on_merged: [US1, US2, US3]
  implements: [FR-013, FR-014, FR-015, FR-016]
US5:
  depends_on: []
  depends_on_merged: [US3, US4]
  implements: [FR-017, FR-018, FR-019, FR-020]
US6:
  depends_on: []
  depends_on_merged: [US5]
  implements: [FR-021, FR-022]
US7:
  depends_on: []
  depends_on_merged: [US5, US6]
  implements: [FR-023, FR-024, FR-025]
US8:
  depends_on: []
  depends_on_merged: [US6, US7]
  implements: [FR-026, FR-027, FR-028, FR-029]
```

Every edge is a merge edge, because each dependent builds on what its predecessor
creates. US3 implements US2's `Steered` for the tracker. US4 creates `Steering` in
US2's file over the tracker US3 made steerable, and its US4-S5 reads US1's memory. US5
adds to US4's `steering.rs` and to US3's `tracking.rs`; US6 adds to both again, after
US5. US7 wires the finished steering into 035's stack and writes the README after US5.
US8 adds the readout's figures to `steering.rs` after US6 and shows them after US7's
block of the stack exists. Where a story names two predecessors and one already
follows the other, the second edge is there because both wrote a file the story
writes. US1 touches no file in common with US2 or US3 (`crates/simulator` against
`crates/finder` and `GLOSSARY.md`) and may run beside them.
