---
state: ready
depends_on_landed: ["013-live-finder", "014-goto", "024-mount-link-recovery", "027-finder-tests-by-system", "038-track-follows-a-push", "047-console-uses-the-database-that-fits", "051-tools-page-focuses-either-camera", "052-node-keeps-saturated-stars"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of that day ("Your
# rec. Proceed", to the operator seat's ranked list of a field night's defects), the
# second of four specs (052 detection keeps saturated stars, 053 this one, 054 the imaging
# run's focus pause, 055 nudge buttons). Folded the same day from a cynical review
# (round 1: 0 blockers, 6 majors, 12 minors) and the seat's direction to re-centre the
# spec on the tilt fix: the draft's hold of far solves is now owner option 1(a), not
# built here. From a probe on the branch `probe/053` cut from f6d4f5fd (tip 764a47c0;
# plan.md § What was run and what was read). Folded again the same day from probe
# round 1, the three stories built from this text alone on `probe/053-r1` (from
# c9a29b2f, tip a78a3f7a: 0 blocking, 8 clarity defects), each fold proved on that
# branch's commits ea235dfb, 3d2fc4e8 and 8fac1157 (tip 8fac1157). Every `path:line`
# here was read at f6d4f5fd, and those in files 052 changed re-read at 01b2d0f5. The
# owner answered the six decisions on 2026-11-27, each with its recommendation, which
# the scenarios were already written to. 014 is named
# because this spec changes its sync intake, pointing model, GoTo and second sync
# (FR-011, FR-012, FR-013, FR-019, FR-022, quoted in Clarifications); 038 because its
# slip mark stays as it is; 013 for the live loop that takes the syncs; 047 for the
# database in use; 024, 027, 038, 047, 051 and 052 because each last changed a file this spec
# writes (plan.md § Refinement Record).
# Re-anchored the same day on `main` at 01b2d0f5, where 052 landed: 052 joins
# `depends_on_landed`, and the record's entries for the README, the glossary and the
# console test `main.rs` name 052's stories. Flipped to `ready` on 2026-11-27 at the
# owner's ask ("Yes, all three", a standing yes to the seat flipping 053, 054 and 055,
# each once the spec before it lands, with the seat's recommended answers), with every
# prerequisite in plan.md met and none waived.
---

# Feature Specification: The Second Sync Fits the Base Tilt

**Feature Branch**: `053-second-sync-fits-base-tilt`

**Created**: 2026-11-27

**Status**: Ready

**Input**: On a field night, on the alt-az mount with the finder camera about 1.6° wide,
GoTos to the Pleiades and M52 had first errors of about 5.0° and 5.7°; no second sync
had been pressed yet. Later the second sync turned the telescope 113° in azimuth at
about 79° up, and the next GoTo, to M42, landed about 5° off (a first error of
17,960″) and ended `failed`, "no frame solved after settling"; the model then read
4 syncs at an RMS of 784″. After the night's model was reset, with no second sync after
it, it read 5 syncs at 2,282″. The operator seat's triage (outside the repository) found,
by reading the code and by a Python copy of the model, that no base tilt was ever
fitted, because a second sync more than 2.5° from its one-sync prediction counts as a
slip and opens a new zero-point epoch (`crates/finder/src/model.rs:508`,
`crates/finder/src/model.rs:515`); that a base tilt of about 3° reproduces the misses;
and that the second sync's spot has no altitude cap (`crates/finder/src/goto.rs:280`),
so one sync near the zenith puts several times the tilt into the azimuth zero. It also
found that nothing checks a solve's field before it becomes a sync
(`crates/finder/src/live.rs:265`, `crates/finder/src/live.rs:274`,
`crates/finder/src/session.rs:1507`), and that a GoTo corrects from `syncs.last()`
(`crates/finder/src/goto.rs:703`). The owner asked for a sync to be refused when it is
far from the model's prediction and for a false match not to be offered as a sync. Make
the second sync fit the base tilt from between 30° and 60° up and say so when it fits
none, take no sync of a field the database in use was not built for, and have a GoTo
correct only from a solve the model took. Only the M42 miss came after a second sync;
the earlier misses and the reset model change only when the second sync is pressed
before the first far GoTo, which the README now says.

## Clarifications

### Session 2026-11-27 (the probe on `main` at f6d4f5fd)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch, logs, the field night's triage and the cynical review are not in the
  repository; what they showed is written here.
- Q: How does a live solve become a sync today? → A: The live loop records the solve as
  the latest pointing and hands it to `Session::record_frame_sync`
  (`crates/finder/src/live.rs:265`, `crates/finder/src/live.rs:274`), which refuses it
  only while the telescope is moving by hand under Track (038,
  `crates/finder/src/session.rs:1516`), without a site or counts, or when
  `mid_exposure` finds an axis moving over 0.2°/s (`crates/finder/src/model.rs:258`);
  `Session::record_sync` calls `Model::add_sync`
  (`crates/finder/src/session.rs:1475`), which refuses only invalid data and a backlash
  crossing (`crates/finder/src/model.rs:493`, `crates/finder/src/model.rs:496`). A sync
  more than 0.5° from the prediction, or 2.5° before the base tilt is fitted, is a slip
  (`crates/finder/src/model.rs:508`), which opens a new epoch fitted from that sync alone
  (`crates/finder/src/model.rs:515`, `crates/finder/src/model.rs:566`). The base tilt is
  fitted only from two syncs of one epoch at least 90° apart in solved azimuth
  (`crates/finder/src/model.rs:531`). The RMS only weights the fit
  (`crates/finder/src/live.rs:268`).
- Q: Does anything check a solve's field? → A: Only the next solve's search, and only
  when there was a previous solve. The live loop solves the next frame within 5 % of the
  last solve's field (`crates/finder/src/live.rs:252`, the estimate set at
  `crates/finder/src/live.rs:264`) and drops that estimate after three misses in a row
  (`crates/finder/src/live.rs:284`, `crates/finder/src/live.rs:285`). Without it, every
  database is tried (`crates/finder/src/databases.rs:133`), and a match switches the
  database in use to the one whose range holds the solved field
  (`crates/finder/src/databases.rs:145`). So at the start and after misses, when a
  false match is likeliest, nothing checks the field, and a false field then pulls the
  next frames toward its own scale. The database's stated range does not depend on a
  previous solve; FR-007 checks the field against it.
- Q: What does this change of 014, 024, 032, 038, 047 and 050? → A: It changes five of
  014's requirements and keeps the others:
  - 014 FR-011, "The console SHALL take as a sync each solved star frame from the scope
    node that carries axis positions, when both axes moved at no more than 0.2°/s during
    its exposure.": now also only when its field lies within 5 % of the range of the
    database in use (FR-007).
  - 014 FR-012, "the base tilt (AN, AW) once it holds two syncs at least 90° apart in
    azimuth": now 90° on the mount's azimuth axis, within one epoch, and up to 6°
    (FR-001, FR-002).
  - 014 FR-013, "Once the base tilt is fitted, a sync more than 0.5° from the model's
    prediction SHALL open a new zero-point epoch. … Before then, a sync SHALL open one
    only when it is more than 2.5° from the prediction (0.5° plus twice a 1° base
    tilt).": the bounds stay, but before the base tilt is fitted a sync at least 90° on
    the azimuth axis from one of the epoch's is judged by the tilt it fits alone, joining
    the epoch at up to 6° and opening one above it (FR-001). The 2.5° rested on a 1° tilt;
    with tilts to 6° admitted at a tilt spot, a far solve before the second sync that is
    not at one still opens an epoch, as today (Edge Cases).
  - 014 FR-019, "It SHALL then solve at the aim point and correct, at most 5 times, until
    the target is within 1′ of the aim point.": the solve it measures the error from is
    one the model took as a sync, and a timeout after solves not taken names the last
    (FR-004, FR-006).
  - 014 FR-022, "the console SHALL slew to a pointable spot at least 90° in azimuth from
    the night's syncs and outside the Sun guard, solve there, and take the sync": the spot
    is looked for first between 30° and 60° up (FR-008), and a second sync that leaves no
    base tilt ends `failed` (FR-009), where it ended `done`.
  - 038 FR-007, "The next sync recorded SHALL open a new zero-point epoch whatever its
    distance from the model's prediction": kept as it is (FR-003).
  - 024 FR-014, the refusals "until the console records a sync from a frame whose
    exposure started after it saw the rise": kept; a solve of a field outside the range is
    no sync, so it does not end the refusal (Edge Cases).
  - 032's owner decision 3 rested on 014 FR-013 for a moved aim point: unchanged.
  - 047's choice of the database in use is unchanged; FR-007 reads its range.
  - 050 says nothing about syncs but that its rig waits until "the session's pointing
    model holds a sync" (`specs/050-tools-page-aligns-finder-camera/spec.md:404`):
    unchanged.
- Q: Did the probe reproduce the field night? → A: Yes, on the factory's rig: the
  simulator's mount carries an exact base tilt and zero points
  (`crates/simulator/src/mount.rs:18`, `crates/simulator/src/mount.rs:216`). With the
  first sync at 75° up and azimuth 122°, on `main` the second sync opened a second epoch
  and fitted no tilt for every tilt of 2° to 5° tried (toward 195° and 300°), and the
  next GoTos' first errors were 7,533″ to 45,412″ (a 3° tilt toward 195°: 17,196″ to a
  place 36° up at azimuth 127°, against the field night's 17,960″ to M42); at 0° and 1°
  the tilt was fitted and the first errors were 51″ to 62″ (run). This spec's build
  fitted the tilt in one epoch, within 1′, at every tilt from 0° to 5° in both
  directions, and every first error was under 120″ (run, US2-S1).
- Q: Why measure 90° on the azimuth axis? → A: The one-sync model's azimuth zero is off
  by several times the tilt when the sync is high, so two syncs a quarter turn apart on
  the mount can land under 90° apart in solved azimuth, and then no tilt is fitted. In the
  draft probe's first trial, with the pairs read in solved azimuth, a 5° tilt toward 45°
  or 90° from a first sync 75° up put the two solved azimuths 87.6° and 89.3° apart, and
  each second sync opened an epoch (run). The mount's own azimuth axis turns exactly as
  commanded whatever the model's error, and what fixes the base tilt is how far apart the
  two directions are in the mount's own frame. With the pairs read on the axis, 56 of 56
  trial cases fitted the tilt in one epoch: 1°, 3° and 5° of tilt, eight directions each,
  from 45°, 75° and 82° up (1° from 75° only), the solved azimuths as little as 51.7°
  apart (run); at 8°, 8 of 8 fitted none.
- Q: Why between 30° and 60° up? → A: The simulator's mount has no collimation (CA) or
  axis-perpendicularity (NPAE) error, so on it two syncs a quarter turn apart fit the tilt
  exactly wherever the second one is. With either term left unmodelled, a high second
  sync leaves more. On a truth written in this spec's own test (US1-S4), with 0.25° of CA
  and a 3° tilt toward 195° after a first sync 77° up, the worst miss over targets 22° to
  82° up is 0.81° with the second sync 81° up and 0.35° with it 47° up (0.41° at 62°);
  with 0.25° of NPAE, 0.95° and 0.42° (0.52° at 62°); at 0.5° the band roughly halves it
  again from 1.63° to 0.71° (CA) and from 1.89° to 0.83° (NPAE) (run). The review's run
  of the model's code agrees to 0.01° on every one of those rows. Both terms together
  depend on their signs: the draft's Python model, with 0.5° of each and a 3° tilt, left
  1.90° to 2.07° worst (median 1.03° to 1.06°) with the second sync in the band and
  3.66° at 79°; with 0.25° of each and a 1° tilt, 0.87° to 0.98° (median 0.52° to 0.54°)
  in the band and 1.78° at 79° (run, outside the repository). The survey puts CA at
  "Arcminutes to tens of arcminutes" (`docs/research/2026-10-21-pointing-model-and-tracking.md:35`).
  SynScan's own alignment advice is 15° to 60° up and 45° to 135° apart
  (`docs/research/2026-10-21-pointing-model-and-tracking.md:101`).
- Q: What is tested, and how independent is it? → A: The simulator's truth is
  independent code of the model's own family: zero points added to the axis directions,
  then a rotation about a horizontal axis (`crates/simulator/src/mount.rs:216`), which
  the model fits exactly from two syncs. So on that rig a tilt within 1′ and a prediction
  within 1″ follow whenever the sync is taken; the GoTo scenarios test which solve is
  taken and what the GoTo and second sync then do, not the fit's robustness. The model
  scenarios add a truth the model cannot fit exactly at two syncs, with CA and NPAE
  (US1-S4), and pin a slip that two syncs cannot tell from a tilt (US1-S5).
- Q: Does a false solve happen? → A: Rarely. A match passes a binomial false-alarm test
  of 1e-5 over the database's pattern count (`crates/finder/src/solve.rs:26`); the
  triage's 600 random subsets and orders of the field night's 69 centroids gave no match
  in either database (outside the repository). The field night's far reading, logged
  that night as a false solve, was most likely a true solve through a model 5° wrong: the
  correction slew that followed it left M42 0.3° from the centre (the triage). A false
  solve of a right-sized field is still taken (owner decision 2); one of a field the
  database was not built for is not (FR-007).

### Session 2026-11-27 (the flip)

- Q: How were the six open decisions answered? → A: Each with its recommendation, by
  the owner on 2026-11-27, who answered "Yes, all three" to the seat's question whether
  it should flip 053, 054 and 055 itself, each once the spec before it lands, with its
  recommended answers.
- Q: What moved on `main` between the draft and the flip? → A: 052 landed, at
  01b2d0f5. Of the files this spec writes or cites, it added a paragraph to
  `crates/finder/README.md` before the GoTo paragraph (both README citations moved by
  five lines), the entry **Saturated star** to `GLOSSARY.md`, one mod line to
  `crates/finder/tests/console/main.rs`, and changed `crates/finder/src/solve.rs`
  away from its cited line. Nothing this spec requires or tests changed.

## Owner decisions

All six are settled by the owner on 2026-11-27, each answered with its recommendation:
(d) for decision 1, (a) for decisions 2, 4, 5 and 6, and for decision 3 (a) for
parts 1 to 5 and (iii) for part 6. Each keeps its options and its evidence; the scenarios and the record were
written to the recommendations, so no scenario changed for them. The general rule behind them is the
owner's own preference: fix the night's defect by the one rule that removes it, the
second sync's solve fitting the base tilt, and add no state to hold or confirm solves.

1. **The intake rule** (FR-001 to FR-003). (a) **The draft's hold**: every solve the model
   cannot vouch for is held until the next solve agrees. It fixes no observed failure: on
   the reviewer's run P, the same false field over 2 or 6 frames after a GoTo's slew was
   taken one frame later all the same (errors 101,828″ then 114,453″, a slew of about 32°
   the wrong way, before a true solve ended it `done` at 9.7″), and this spec's build
   without the hold gave the very same numbers (run); only two alternating false fields,
   never seen, are refused by it, where this build takes them and recovers (errors
   200,538″ and 227,544″, then 45″, `done`; run). On a truth with 0.25° of CA and a second
   sync at 79° up, 24 of 24 true solves 30° up were held (the review's run U), so a GoTo
   would need two solves in its first 30 s where `main` needs one. Size and approvals: the
   draft's model story, the hold with the tilt fix, measured 19.0 KB on its probe against
   this spec's 15.5 KB for the tilt fix with its eight scenarios; one line of an existing
   unit test replaced, 014's FR-011 and FR-013 changed for it, a `model.candidate` key and
   a glossary entry. (b) **A gate of its own at 1°**, a second solve within 5′ to confirm:
   the same weakness, and two bounds. (c) **No gate and no tilt fix**, as today: the
   night's defect stays (run: log 02). (d) **The tilt fix and the band only**, with
   decision 4's field check and decision 3's plain end, and the hold left as a later spec
   the owner may decline. Size: the three stories here, 15.5, 28.3 and 15.4 KB; approvals:
   014's FR-011, FR-012, FR-013, FR-019 and FR-022 changed as quoted, two optional `goto`
   keys, no existing test edited. Answered: **(d)**.
2. **How a real slip is told from a bad solve** (FR-003). (a) **As today: a sync beyond
   the bound opens an epoch at once, and the next true solve opens another**; a push by
   hand while Track runs keeps 038's mark; a mount reset needs nothing of its own (024's
   refusal ends on the first sync after the rise; `crates/finder/tests/motion/mount_identity.rs`
   passed unchanged, run). Evidence: run P above, the same with and without a hold.
   (b) **Persistence** (decision 1(a)): no better on a repeated false field. (c)
   **Confirmation after a move**, the next correction's solve: not measured, and a
   correction planned from a false solve moves the telescope before it is confirmed.
   Answered: **(a)**, with decision 4's field check as the independent test against a
   false match.
3. **The second sync** (FR-001, FR-008, FR-009).
   1. *Its band.* (a) **Between 30° and 60° up as the model predicts it, the altitudes
      searched as today; the nearest outside it only when no spot in the band has a safe
      path** (Clarifications: why 30° to 60°). (b) 15° to 60°,
      SynScan's own: more trees and more refraction low down. Answered: **(a)**.
   2. *Its tilt bound.* (a) **6°**, above every tilt tried (0° to 5°, fitted within 1′);
      8° is taken as a slip (US1-S2, US2-S3). (b) **5°**, the top of the range tried and
      too close to it for a fit that lands at 5.00°. Immaterial to slips: two syncs fit
      the four terms exactly, so the bound is no check on a slip between them (part 6).
      Answered: **(a)**.
   3. *A blocked spot.* (a) **It fails as today, "no frame solved after settling"; the
      README says that a GoTo to a clear target at least 90° round on the azimuth axis
      fits the base tilt by the same rule**. Run: with no second sync, a GoTo to a place
      40° up at azimuth 250° from a first sync 75° up at 122° fitted every tilt of 1°, 3°
      and 5°, both directions, in one epoch within 0.011°, `done` after one correction,
      and the next far GoTo's first error was 54″ to 62″. (b) **Skip a spot that timed
      out, or try the mirror spot**: pressing the second sync again plans the same spot,
      since the mount sits on it and the planner starts from the current place
      (`crates/finder/src/goto.rs:280`, `crates/finder/src/goto.rs:293`); a rule that
      needs the timed-out spot kept. Answered: **(a)**.
   4. *The level-mount change.* With the band, a level mount's second sync from 75° up
      goes to 60° up, and from 20° up to 30° up, where today it stays at the current
      altitude (US2-S2). (a) **Accept it**: the band is what halves the unmodelled terms'
      share. (b) Keep the current altitude on a level mount: no way to know the mount is
      level before the tilt is fitted. Answered: **(a)**.
   5. *A second sync that fits no base tilt.* (a) **It ends `failed` with
      `taken as a slip: no base tilt fitted`** (FR-009), where today it ends `done`,
      fitting nothing, as on the field night. The page shows the reason after "GoTo
      failed" (`crates/finder/web/src/components/GoToControl.tsx:19`). (b) **`done` with
      that reason**: the page would read "On target" before it. Answered: **(a)**.
   6. *A check before a two-sync tilt is trusted.* Two syncs fit IA, IE, AN and AW
      exactly, so a slip between them is taken as a wrong tilt: on the reviewer's run S,
      3° to 8° slips left worst misses of 3.5° to 9.4° and 92 % to 99 % of the sky over
      0.5°, all night. `main` opens an epoch for a slip over 2.5° from the one-sync
      prediction; this rule takes it at a tilt spot (US1-S5 pins a 5° one). Measured on a
      truth written in the test, five pairs of syncs (plan.md § What was run): (i) **the
      roll difference between the two syncs**, the survey's third equation from a solve
      with roll (`docs/research/2026-10-21-pointing-model-and-tracking.md:41`,
      `docs/research/2026-10-21-pointing-model-and-tracking.md:48`), in which the camera's
      own rotation cancels: a true tilt with up to 0.5° each of CA and NPAE mismatches it
      by 0.30° at most, azimuth slips of 1°, 3°, 5° and 8° by 1.02° to 1.47°, 3.08° to
      4.43°, 5.12° to 7.43° and 8.19° to 12.00°, but altitude slips of 1° to 5° by 0.03°
      to 1.02° only; and on the factory's simulator, whose camera keeps its top to the
      true vertical whatever the base tilt (`crates/simulator/src/sky.rs:106`), true tilts
      of 1° to 5° mismatch it by 2.5° to 22.5° (0.46° level), so it cannot pass the
      factory's rig before the simulator's camera rolls with a tilted base. (ii) **The
      tilt provisional until a third solve agrees within 1°**: absorbed slips of 3° or
      more leave 43 % to 99 % of places over 1°, a true tilt with 0.5° of CA or NPAE and
      the second sync in the band 0 % to 6 %; but it cannot tell a slip before the tilt
      from one after it, so the first push by hand with Track off after the second sync
      would drop a true tilt, and a catch leaves the model with no tilt at all. (iii)
      **None, as an accepted risk**: a slip over 2.5° while the telescope stands still is
      taken by the next live solve as a slip epoch before the second sync measures from
      it, so the window is the second sync's own slew; the README says to press the second sync at
      once, with the telescope untouched. Answered: **(iii) here, and (i), the simplest
      that catches most slips, as a later spec with the simulator's camera rolling with
      the base first**.
4. **The field check** (FR-007). (a) **A live solve whose field lies more than 5 %
   outside the range of the database in use, once the frame's tries are done, is not
   taken as a sync**; the solve is still the live pointing. With several databases, 047
   has then chosen the one whose range holds the field, so the check refuses a field in no
   database's range. It holds with no previous solve, when the live loop's own 5 % bound
   does not (Clarifications: does anything check a solve's field). Run: US2-S4 and US2-S5,
   a solve answered at 0.7 times its field. (b) **The same check in the live loop**, the
   solve then no pointing either: a change to `crates/finder/src/live.rs` and to 047's
   lens-change path, where a true solve by the wrong database is what switches the
   database. (c) **Also at least 8 matched stars**: a number nothing measured supports.
   (d) **None**, as today. Answered: **(a)**.
5. **What the page and the API show** (FR-005, FR-006, FR-009). (a) **`goto.frame_id` and
   `goto.frame_time` in `GET /api/session`, and so in MCP's `get_session`
   (`crates/finder/src/mcp/tools.rs:538`, `crates/finder/src/api.rs:884`): the frame
   whose sync gave the error or ended the second sync, and its mid-exposure, so its age is
   the session's `time` less `frame_time`; the two failure reasons of FR-006 and FR-009.
   No page change**: the page shows a GoTo's reason
   (`crates/finder/web/src/components/GoToControl.tsx:19`), "Base tilt needs two syncs at
   least 90° apart." while none is fitted
   (`crates/finder/web/src/components/MountSection.tsx:38`), and "Pointing stale" for a
   live pointing over 5 s old (`crates/finder/src/pushto.rs:184`,
   `crates/finder/web/src/components/ChartHeader.tsx:73`). (b) **Also a page line**: a
   page story, its build and its browser test, for words the reason already gives. No
   held state is left to count or mark. Answered: **(a)**.
6. **A warning while the model rests on one sync above about 70°** (none). (a) **None**:
   the band roughly halves what such a first sync leaves (Clarifications), the warning
   that mattered on the night was the silent second sync, which now fails plainly
   (decision 3, part 5), and the README says to take the first sync between 30° and 60°
   up when the sky allows. (b) **`model.warning` and a Setup line**. Answered: **(a)**.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No golden, fixture, CI file or existing
test changes.

1. **New public API in `crates/finder`'s library** (FR-005): the fields
   `goto::Progress::frame_id` and `goto::Progress::frame_time`. No existing signature
   changes.
2. **Two new optional keys in `GET /api/session`** (FR-005): `goto.frame_id` and
   `goto.frame_time`, each absent before a GoTo or second sync takes a sync.
3. **Five of 014's requirements changed** as Clarifications quotes them: FR-011, FR-012,
   FR-013, FR-019 and FR-022; a second sync that fits no base tilt now ends `failed`.
4. **The glossary** (FR-013): **Base tilt** and **Slip** reworded.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process.
- **The mount's geometry**, the oracle of every Rust scenario, written fresh in each test
  file: axis directions (azimuth a, altitude e, in degrees) plus zero points; in the model
  scenarios also the collimation c and the non-perpendicularity n (both 0 unless a
  scenario says otherwise), with the altitude axis h = (−sin a cos n, cos a cos n, sin n),
  f = (cos a, sin a, 0) and the tube cos c (cos e f + sin e (f × h)) + sin c h, in
  north, east and up; then rotated by the base tilt t about the horizontal axis
  (−sin w, cos w, 0), w the azimuth it leans toward, with Rodrigues' formula, as
  `simulator::mount` defines the true direction (`crates/simulator/src/mount.rs:216`).
  No expected value comes from the code under test: no test calls `finder::model`'s
  `separation` or the fit for an expected value, or reads a constant of the code; each
  test writes its own great-circle distance by the chord. A test may call `predict` only
  to measure the model's miss against the geometry. The support rig's `finish_with` and
  `error_arcsec` measure the Sun distance and the true error with the model's
  `separation` (`crates/finder/tests/support/goto.rs:331`,
  `crates/finder/tests/support/goto.rs:344`), as in every GoTo test.
- **The model rig** is `finder::model::Model::default()` fed `finder::model::Sync`s that
  the test builds: **a sync at (a, e) at time T** has the axis positions a and e at
  1,620,000 counts a revolution, the azimuth and altitude the mount's geometry gives them
  under zero points (17°, −2°) unless a scenario says otherwise, RMS 1″, no roll and epoch
  0. **The worst miss** of a model is the largest chord distance between its prediction
  and the geometry at the axis directions of azimuths 0° to 345° every 15° and altitudes
  22° to 82° every 10°.
- **The GoTo rig** is `crate::support::goto::Rig`'s fields built in the test file over a
  simulator on `crate::support::live::DB`, cedar's database of a 10° to 30° range, at
  `crate::support::live::START`, a `SimMount` of the scenario's `MountConfig` (backlash
  0.3°) connected to a `NodeService` with `crate::support::goto::Recorded`, a 500 ms
  frame interval, the session `Rig::new` edits (site, target M42, 200 ms at gain 1, the
  aim point (700, 420)) and a live solver: `crate::support::goto::Stars`, or **Scale**,
  which solves each frame as `Stars` does but with no field estimate and, for the frames
  whose ids lie in a window the test sets, answers the solved place with a frame of 0.7
  times the solved field (about 8.4° for the simulator's 12°), built with
  `finder::sky::Frame::from_solution` from a `plate_solver::Solution` made of the
  report's `pointing` (its ra, dec, roll and distortion, and a camera of the frame's size
  at 0.7 times its `fov_deg`). **The tilt rig at (alt, az)** is the GoTo rig with
  `Stars` over a tilt t toward w with zero points that put counts (4500, 4500), reached
  from rest at zero, at the true (alt, az): the test's own inverse rotation, the tube
  0.85° after half the backlash, as `crates/finder/tests/motion/goto/second_sync.rs`'s
  `initial_zero` computes it; moved to (4500, 4500), the night's model reset and the
  first sync awaited with `Rig::sync`. **The level rig** is the tilt rig with t = 0.
  **The field rig** is the GoTo rig with Scale over a tilt of 1° toward 130° with zero
  points (120°, 50°), moved to (4500, 4500) and then to (580000, 4500), so that the model
  holds one epoch and a base tilt. **A place (alt, az)** is the catalog's M42 moved to
  that true horizontal place at the rig's time, as
  `crates/finder/tests/motion/goto/sim.rs`'s `horizontal_target` makes it. **The two
  places** are (36°, 127°) and (40°, 250°). The tests run under
  `#[tokio::test(start_paused = true)]`, and every GoTo and second sync is read with
  `Rig::finish_with(Some(15.0), true, …)`, which asserts at every read that the true
  altitude is at least 15° and the true direction at least 15° from the Sun, and that it
  ends within 5,000 reads.
- **The docs rig** is `crates/finder/README.md` and `GLOSSARY.md` read from
  `env!("CARGO_MANIFEST_DIR")` (the glossary as `../../GLOSSARY.md`), as
  `crates/finder/tests/console/imaging_run_docs.rs` reads the README, with every run of
  whitespace joined into one space (`split_whitespace`, joined by one space).

### User Story 1 - The second sync's solve fits the base tilt (Priority: P1)

As the owner, I need the pointing model to take the second sync's solve, a quarter turn
round on the mount's azimuth axis, as the measurement of the base tilt that it is, so that
a base a few degrees off level is fitted instead of read as a slip.

**Why this priority**: The field night's misses come from the second sync taken as a
slip, which left the model with no base tilt.

**Independent Test**: `crates/finder/tests/motion/model/tilt.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test motion -E 'test(/^model::tilt::/)'`.

**Acceptance Scenarios**:

1. **Given** the model rig with a sync at (105°, 77°) at 0 s and a sync at (0°, 45°) at
   120 s, under tilts of 0°, 1°, 3° and 5°, each toward 195° and toward 15°, **When** a
   committed test adds the two in turn for each tilt and direction, **Then** it asserts
   that for 5° toward 15° the geometry puts the two syncs' azimuths under 90° apart, that
   both `add_sync` calls answer true, that the model holds one epoch and a base tilt whose
   `deg()` is within 1′ of the tilt and, above 0°, whose `toward_deg()` is within 1′ of
   the direction, and that `predict` at the axis direction (250°, 30°) lies within 1″ of
   the mount's geometry.
2. **Given** the model rig under a tilt of 8° toward 195° with the two syncs of US1-S1,
   **When** a committed test adds them, **Then** it asserts that both answer true, that
   the model holds two epochs and no base tilt, and that `predict` at (0°, 45°) lies
   within 1″ of the geometry there.
3. **Given** the model rig under a tilt of 7° toward 0° with a sync at (120°, 10°) at 0 s
   and one at (210°, 10°) at 120 s, **When** a committed test computes the one-sync
   prediction of the second itself (its axis directions plus the first sync's azimuth and
   altitude offsets) and adds the two, **Then** it asserts that the one-sync prediction
   lies within 2.5° of the geometry, that both answer true, and that the model holds two
   epochs and no base tilt.
4. **Given** the model rig under a tilt of 3° toward 195°, once with a collimation of
   0.25° and once with a non-perpendicularity of 0.25°, a sync at (105°, 77°) at 0 s and
   a second at 120 s at (0°, 47°) or at (0°, 81°), **When** a committed test adds the two
   for each of the four cases, **Then** it asserts that both answer true, that the model
   holds one epoch and a base tilt whose `deg()` is within 0.15° of 3°, and that the
   worst miss is under 0.5° with the second sync at (0°, 47°) and not under 0.5° with it
   at (0°, 81°).
5. **Given** the model rig under a tilt of 1° toward 130°, a sync at (30°, 40°) at 0 s
   and a sync at (140°, 50°) at 200 s under zero points (22°, −2°), a slip of 5° in
   azimuth between them, **When** a committed test computes the one-sync prediction of
   the second itself, as in US1-S3, and adds the two, **Then** it asserts that the
   one-sync prediction lies more than 2.5° from the slipped geometry, that both answer
   true, that the model holds one epoch and a base tilt, and that the worst miss against
   the slipped geometry is over 2°.
6. **Given** the model rig under a tilt of 1° toward 130°, a sync at (30°, 40°) at 0 s,
   `Model::mark_slip`, and a sync at (140°, 50°) at 200 s, **When** a committed test adds
   them, **Then** it asserts that both answer true and that the model holds two epochs
   and no base tilt.
7. **Given** the model rig under a tilt of 8° toward 195° with the syncs of US1-S2 and a
   third at (350°, 45°) at 240 s, **When** a committed test adds the three, **Then** it
   asserts that the third answers true and the model still holds two epochs.
8. **Given** the model rig under a tilt of 1° toward 130° with a sync at (350°, 45°) at
   0 s and one at (20°, 45°) at 120 s, **When** a committed test adds them, **Then** it
   asserts that both answer true and the model holds one epoch and no base tilt.

### User Story 2 - The second sync keeps to the band and says when it fits no tilt, and a GoTo corrects only from a taken sync (Priority: P1)

As the owner, I need the second sync to go where it can measure the base tilt, between
30° and 60° up, and to fail plainly when it fits none; a solve of a field the database in
use was not built for not to be taken as a sync; and a GoTo to correct only from a solve
the model took, naming that frame.

**Why this priority**: The model's rule is only as good as where the second sync goes and
what a GoTo corrects from, and a silent second sync was the night's core failure.

**Independent Test**: `crates/finder/tests/motion/goto/tilt.rs` and
`crates/finder/tests/motion/goto/field.rs`, new. The `test` gate's nextest command runs
them; during development,
`cargo nextest run -p finder --locked --test motion -E 'test(/^goto::tilt::/) | test(/^goto::field::/)'`.

**Acceptance Scenarios**:

1. **Given** the tilt rig at (75°, 122°) for tilts of 0°, 1°, 2°, 3°, 4° and 5°, each
   toward 195° and toward 300°, **When** a committed test sends
   `POST /api/model/second-sync` and then starts a GoTo to each of the two places in
   turn, **Then** it asserts that the second sync ends `done` with every progress read at
   correction 0, that the model then holds two syncs, one epoch and a base tilt whose
   `deg()` is within 1′ of the tilt, that each GoTo ends `done` with its first
   `error_arcsec` under 120, a `frame_id` above the latest frame's id when it was started
   and a `frame_time` after the console time when it was started, that every progress
   read of each GoTo in state `slewing` at correction 0 has no `frame_id`, and that the
   model still holds one epoch after both.
2. **Given** the level rig at (75°, 122°), at (45°, 122°) and at (20°, 122°), **When** a
   committed test sends `POST /api/model/second-sync` on each, **Then** it asserts that
   each ends `done`, that the mount's true altitude is then within 0.2° of 60°, 45° and
   30° respectively, that the azimuth axis turned at least 90° and the true azimuth
   changed by at least 90°, and that the model holds one epoch.
3. **Given** the tilt rig at (75°, 122°) for a tilt of 8° toward 45°, **When** a
   committed test sends `POST /api/model/second-sync`, **Then** it asserts that the
   second sync ends `failed` with the `reason` `taken as a slip: no base tilt fitted` and
   every progress read at correction 0, and that the model then holds two epochs and no
   base tilt.
4. **Given** the field rig, **When** a committed test starts a GoTo to M42 and, at the
   first progress read `solving`, sets Scale's window to the frames latest + 1 to
   latest + 4, latest being the latest frame's id then, **Then** it asserts that the GoTo
   ends `done`, that its `frame_id` is above latest + 4, that the first `error_arcsec`
   read is under 120, that the final `error_arcsec` and the simulator's true error from
   M42 (`Rig::error_arcsec`) are each under 60, that a `Session::snapshot` read in
   `finish_with`'s sample had a `field.fov_deg` under 9, and that the model holds the
   same number of epochs as before.
5. **Given** the field rig, **When** a committed test starts a GoTo to M42 and, at the
   first progress read `solving`, sets Scale's window from the frame after the latest
   without an end, **Then** it asserts that the GoTo ends `failed` with the `reason`
   `frame <id> solved but was not taken as a sync`, its id at least the window's first
   frame, with no `error_arcsec`, that the id it names is within 3 of the latest frame's
   id when it ends, that it ends within 35 s of the first progress read `solving`, and
   that the model holds the same number of epochs as before.

### User Story 3 - The README and the glossary say how the second sync fits the tilt (Priority: P2)

As the owner, I need the README and the glossary to say which syncs fit the base tilt,
where the second sync goes, what it and a GoTo say when they fail, which solves are not
taken, and how to take the first two syncs, in the words the console uses.

**Why this priority**: The night's use of the second sync depends on knowing them; the
code is already right without them.

**Independent Test**: `crates/finder/tests/console/model_docs.rs`, new. The `test` gate's
nextest command runs it; during development,
`cargo nextest run -p finder --locked --test console -E 'test(model_docs)'`.

**Acceptance Scenarios**:

1. **Given** the docs rig, **When** a committed test reads `crates/finder/README.md` and
   `GLOSSARY.md`, **Then** it asserts that the README holds each of FR-012's ten
   sentences and the glossary each of FR-013's two, word for word.

### Edge Cases

- **A mount reset** (the node's `mount_resets` rises, `crates/finder/src/session.rs:1445`):
  the first solve after it opens a new epoch, as today, and ends 024's refusal
  (`crates/finder/src/session.rs:1558`). Run: every test of
  `crates/finder/tests/motion/mount_identity.rs` passed unchanged.
- **A push by hand while Track runs**: 038 marks the slip
  (`crates/finder/src/session.rs:1302`) and the first sync after it opens an epoch, at a
  tilt spot too, before its tilt is judged (FR-001, US1-S6). Run: the `tracking::push*`
  and `steering_*` tests passed unchanged.
- **A push by hand with Track off**: the first solve once the telescope is still is
  more than the bound from the prediction and opens an epoch, as today; a later second
  sync measures from that epoch's syncs. Run:
  `hand_push_opens_an_epoch_and_goto_still_converges`
  (`crates/finder/tests/motion/goto/sim.rs:159`) passed unchanged.
- **A slip during the second sync's own slew**: taken as tilt, an accepted risk (owner
  decision 3, part 6; US1-S5).
- **The same false solve of a right-sized field twice in a row**: taken, and the next
  true solve opens another epoch (owner decision 2; run P: `done` at 9.7″ after two
  corrections).
- **A solve of a field outside the range** (FR-007): the live pointing, not a sync; it
  does not end 024's refusal after a reset, and a GoTo passes over it (US2-S4). Track
  reads the live pointing, as today.
- **A tree at the second-sync spot**: no frame solves and it fails as today. A GoTo to a
  clear target at least 90° round on the azimuth axis then fits the tilt by FR-001 (run:
  owner decision 3, part 3). A push by hand does not, since it is a slip.
- **A GoTo before the second sync** to a target under 90° round on the azimuth axis:
  its first solve more than 2.5° off opens an epoch, as today.
- **A base more than 6° off level**: the second sync is taken as a slip and ends `failed`
  (US1-S2, US2-S3); the GoTos correct as before.
- **Thinning**: a later sync within 2° and 120 s of a taken one replaces it
  (`crates/finder/src/model.rs:524`); a report whose sync is gone by the time the GoTo
  reads it is passed over and a later one taken (FR-004).
- **Track running**: the tracker takes live pointings, not syncs
  (`crates/finder/src/session.rs:1178`), and is unaffected. Run: every tracking and
  steering test passed unchanged.
- **The imaging run**: its GoTo and centring are this GoTo. Run: every `imaging_run*`
  test passed unchanged.

## Requirements *(mandatory)*

### Functional Requirements

**The pointing model** (US1):

- **FR-001**: Before the base tilt is fitted, `Model::add_sync` SHALL judge a qualified
  sync at least 90° on the azimuth axis from a sync of the current zero-point epoch by
  the base tilt its taking would fit alone, whatever its distance from the prediction:
  at most 6° (`MAX_BASE_TILT_DEG`), it SHALL join that epoch and fit the tilt unless
  038's slip mark is set (FR-003), which opens an epoch first; above it, it SHALL open a
  new epoch fitted from that sync alone, as 014's FR-013 fits one. **Refs**: 014's FR-012
  and FR-013; 038's FR-007; owner decisions 1 and 3.
- **FR-002**: Two syncs SHALL count as 90° apart for the base tilt when their axis
  positions' azimuths, the azimuth count times 360 over the counts a revolution, differ
  by at least 90°, within one epoch. **Refs**: 014's FR-012; Clarifications (why the
  azimuth axis).
- **FR-003**: Nothing else in the model SHALL change: the night's first sync, 014's
  bounds of 0.5° and 2.5° for every other sync, 038's slip mark taken at once, the
  thinning, the fixed terms, the state file's shape, and no check before a two-sync tilt
  is trusted. **Refs**: 014's FR-011 to FR-015; 038's FR-007; owner decisions 2 and 3.

**The second sync, the field and GoTo** (US2):

- **FR-004**: The second sync's solve and each GoTo's solve SHALL be one the model took
  as a sync: of the reports for frames that started after the settle, the loop SHALL pass
  over one whose time no sync of the session's model holds when it reads the report,
  and wait on within the same 30 s. The error SHALL be measured from the report it takes,
  and each correction planned as today, from the newest sync the model holds
  (`crates/finder/src/goto.rs:1024`, `crates/finder/src/goto.rs:703`). **Refs**: 014's
  FR-019 and FR-022.
- **FR-005**: The `goto` progress SHALL carry `frame_id` and `frame_time`, the id and the
  mid-exposure in console Unix seconds of the frame whose sync gave `error_arcsec` or
  ended the second sync, from the moment it is taken until the next GoTo or second sync
  starts; absent before. **Refs**: owner decision 5; owner approvals 1 and 2.
- **FR-006**: When the 30 s pass after the loop passed over a solved report and took
  none, the GoTo or second sync SHALL end `failed` with
  `frame <id> solved but was not taken as a sync`, naming the last it passed over; with
  none solved, `no frame solved after settling` as today. **Refs**: owner decision 5.
- **FR-007**: `Session::record_frame_sync` SHALL take no sync from a solve whose field
  lies outside the range of the database in use, once the frame's tries are done,
  widened by 5 % at each end (from 0.95 times its lower limit to 1.05 times its upper);
  the solve stays the live pointing. **Refs**: 014's FR-011; 047's database in use;
  owner decision 4.
- **FR-008**: The second sync's spot SHALL be the nearest in turn and tilt (the
  hypotenuse of the azimuth and altitude differences) among the spots whose altitude, as
  the model predicts it, lies between 30° and 60° (`SECOND_SYNC_BAND`), the altitudes
  searched as today; only when none of those has a safe path SHALL it be the nearest
  outside the band, as today.
  Today's 90° on the sky with its allowance (`crates/finder/src/goto.rs:275`) stays.
  **Refs**: 014's FR-022; owner decision 3.
- **FR-009**: A second sync whose taken sync leaves the model with no base tilt SHALL end
  `failed` with `taken as a slip: no base tilt fitted`. **Refs**: 014's FR-022; owner
  decision 3.
- **FR-010**: Nothing else in GoTo or the second sync SHALL change: the settle of 0.5 s,
  the 30 s wait, five corrections to 1′, the states and the refusals. **Refs**: 014's
  FR-017 to FR-023; 024's FR-014 and FR-015.

**The words** (US3):

- **FR-011**: `crates/finder/README.md`'s GoTo and second-sync paragraphs SHALL say what
  FR-001 to FR-009 do, in the words of plan.md § Interfaces.
- **FR-012**: The README SHALL hold these ten sentences, each word for word with its line
  breaks free:
  1. "then for a solved exposure starting after that wait that the pointing model takes
     as a sync, and measures the error from that sync."
  2. "`goto: {state, correction, error_arcsec, reason, frame_id, frame_time}` only after
     one starts, retaining its result until the next GoTo; `frame_id` is the star frame
     whose sync gave `error_arcsec` or ended the second sync, and `frame_time` its
     mid-exposure, so its age is the session's `time` less `frame_time`."
  3. "When 30 s pass after settling with frames solved but none taken as a sync, the GoTo
     or second sync ends `failed` with `frame <id> solved but was not taken as a sync`,
     naming the last; with no frame solved, `no frame solved after settling`."
  4. "It looks first between 30° and 60° up, as the model predicts the altitude, and only
     when no spot there has a safe path does it take the nearest outside it."
  5. "When that sync is taken as a slip and no base tilt is fitted, it ends `failed` with
     `taken as a slip: no base tilt fitted`."
  6. "Before the base tilt is fitted, a sync at least 90° on the azimuth axis from a sync
     of the current zero-point epoch joins that epoch and fits the base tilt when the
     tilt it fits is at most 6°, and is otherwise taken as a slip; the tilt alone decides
     it."
  7. "opens a new zero-point epoch, as does the first solve after a push by hand while
     Track runs, at a tilt spot too."
  8. "A live solve whose field lies more than 5% outside the range of the database in use
     is not taken as a sync."
  9. "take the first sync between 30° and 60° up when the sky allows, and press the
     second sync at once, with the telescope untouched, before the first far GoTo."
  10. "If a tree blocks the second sync's spot, a GoTo to a clear target at least 90°
      round on the azimuth axis fits the base tilt by the same rule."
- **FR-013**: `GLOSSARY.md` SHALL hold these two sentences, each word for word with its
  line breaks free, **Base tilt** and **Slip** reworded (owner approval 4):
  1. "Fitted once two syncs of one zero-point epoch lie at least 90° apart on the azimuth
     axis, up to 6°."
  2. "Before the base tilt is fitted, a sync a quarter turn round on the azimuth axis is
     one only when the tilt it fits is over 6°."

### Key Entities

- **A tilt spot**: before the base tilt is fitted, a sync at least 90° on the mount's
  azimuth axis from a sync of the current zero-point epoch; judged by the tilt it fits.
- **The second sync's spot**: 90° on the sky from the night's syncs, 30° to 60° up as
  the model predicts it when one is reachable.
- **The field check**: the solved field against the range of the database in use,
  widened by 5 %.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No golden, fixture, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or existing test is edited, but the `mod` lines of
  `crates/finder/tests/motion/model/mod.rs`, `crates/finder/tests/motion/goto/mod.rs` and
  `crates/finder/tests/console/main.rs`. Every existing test passes unchanged.
- **SC-003**: On a tilted base of 0° to 5°, the second sync fits the tilt within 1′ in
  one epoch and the next GoTos' first errors are under 120″ (US2-S1), where `main` left
  first errors of 7,533″ to 45,412″ for 2° to 5° (plan.md § What was run).
- **SC-004**: A second sync that fits no base tilt says so (US2-S3), and a solve of a
  field the database in use was not built for neither moves the model nor steers a GoTo
  (US2-S4, US2-S5). A false solve of a right-sized field is still taken (owner decision 2).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd, and those in files 052 changed re-read at 01b2d0f5.
- **The simulator's mount stands for the real one** in the gates: an exact base tilt,
  zero points and backlash, and no collimation, axis-perpendicularity or flexure error,
  and its camera keeps its top to the true vertical (`crates/simulator/src/sky.rs:106`).
  It is independent code of the model's own family, so the GoTo scenarios test which
  solve is taken and what follows, not robustness; the model scenarios add CA and NPAE
  (US1-S4) and a slip (US1-S5), and the real mount's terms are the night's to measure
  (plan.md § Outside the factory).
- **Out of scope:**
  - star detection and saturated stars (052), the solver's matching, the databases and
    the scope node;
  - the imaging run's focus pause (054) and nudge buttons (055);
  - 038's push thresholds, and the cause of the field night's `mount.resets`;
  - the draft's hold of far solves (owner decision 1(a)) and a check before a two-sync
    tilt is trusted (owner decision 3, part 6), each a later spec if the owner wants it;
  - the page: it shows a GoTo's reason and a stale pointing already (owner decision 5);
  - MCP's code: `get_session` returns the session the API returns
    (`crates/finder/src/mcp/tools.rs:538`, `crates/finder/src/api.rs:884`).
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-004, FR-005, FR-006, FR-007, FR-008, FR-009, FR-010]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-011, FR-012, FR-013]
```

Three stories, each under 60 % of 65,536 B (39,321 B) as the probe from this text and
its fold measure them, the largest US2 at 43.2 %, and at the factory's worst overrun
seen, 1.35 times, still under it, US2 at about 58.3 % (plan.md § Plan traps 2): the
model; the second sync, the field and GoTo; the words. Each merges after the one before
(`depends_on_merged`): US2's tilt rig needs US1's tilt rule to fit a tilt over 1° in one
epoch, and US3's words describe US2's progress keys, reasons and band. Plain
`depends_on` would order them without putting the earlier code in the later story's base.
