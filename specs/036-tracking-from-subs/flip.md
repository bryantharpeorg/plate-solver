Not given to builders or the judge. Nothing here is a rule; the trio is.

# Flip: Tracking Steered by the Subs

`036-tracking-from-subs`, the first of two pieces. What the owner settles and approves
when this spec is flipped to `ready`, what must be met first, and what is checked outside
the factory. The trio cites the owner decisions and approvals below by number and states
the rule each one settles. A bare § cite in this file is to plan.md.
`040-stack-waits-for-steady`, the second piece, has its own flip.md and is flipped once
this spec has landed.

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
   (FR-017, FR-018, 040's FR-022; FF5). Not taken: the pause alone; ending the stack with a
   reason of its own.
3. **Where the settings live** (the plan document's route table, in this worktree, has
   this route as 036's row). Recommended: a route of this spec's own,
   `PUT /api/imaging/steering`, run-only as 031's console settings are; the plan
   document's table already has the row, and another answer corrects it (plan.md
   flip.md § Prerequisites 9). Instead: keys of
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
13. **The simulator's count steps.** Recommended: off by default, so no existing
    simulator figure moves. Instead: on by default.
14. **The part's name, its file and its words** (the plan document's list of the parts
    of the session's `imaging` object and its module tree, in this worktree, name
    `steering` and `steering.rs` for 036). Recommended: `steering`, the plan document's
    own verb, for the part of the imaging view and for the module `crates/finder/src/imaging/steering.rs`, with the
    sources `subs`, `tracker` and `none`: the imaging path says "the tracker's own
    corrections" and never "the finder camera", which another telescope may not have.
    The plan document's two lists already carry the name, and another answer corrects
    them (flip.md § Prerequisites 9). Instead: `tracking`, which would give the console
    two files of that name; or the source `finder`.
15. **Which subs steer.** Recommended: only a sub the stack accepted, as the plan
    document has it: a wrong solve that 035 refuses never reaches the mount, and eight
    gaps in a row with no accepted sub hand the tracker back to its own corrections
    (three as drafted: changed by the owner on 2026-11-19 with owner decision 6).
    Instead: also a sub dropped as `soft` or `off_field`, whose solve is as good as any;
    that would let the subs pull back a drift that is itself what made them soft.
16. **How long the simulator remembers.** Recommended: one hour of its clock, the
    simulated camera's longest exposure. Instead: a count of samples.

Decisions 9 to 12 and 17 settle what `040-stack-waits-for-steady` builds; its flip.md
lists them under these numbers.

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
   tracker is not steady, FR-008 holds as written (FR-007, FR-010, FR-011, 040's FR-021). No
   existing test changes.
3. **`PUT /api/imaging/steering`**, the `steering` part of the imaging view and of the
   session's `imaging` object, which the MCP tools that return either carry too, the
   module `crates/finder/src/imaging/steering.rs`, and, since 2026-11-19, the MCP tool
   `set_steering` (FR-005, FR-016, FR-019, 040's FR-031). The plan document's § Interfaces, in
   this worktree, names the first three and no MCP tool for 036; owner decision 11's
   answer of 2026-11-19 corrects its list of tools in the same change as the flip
   (flip.md § Prerequisites 9).
   The MCP tool `set_steering` (040's FR-031) is built by `040-stack-waits-for-steady`, whose
   flip.md approves it under this number.
4. **Two `GLOSSARY.md` terms**, Tracking error and Set point (FR-006).
5. **The simulator's imaging truth changes for a moving mount** (FR-001): 030's
   `imaging_truth_at` then answers with the direction the telescope had at the instant,
   and as before on a still mount. And the simulated mount gains count steps, off by
   default (FR-002).
8. **A route row** in the `## API` table of `crates/finder/README.md` (FR-020). The
   README section and the lines of the MCP tool (040's FR-025, 040's FR-031) are 040's, approved by
   its flip under this number.
9. **Each new test module is declared in an existing test file** (SC-002): one `mod`
   line a module, in alphabetical order, in `crates/finder/tests/console/main.rs`
   (`steering_cap`, `steering_error`, `steering_gap`) and
   `crates/finder/tests/motion/main.rs` (`steering`, `steering_cap`, `steering_claim`,
   `steering_gap`), as 027's layout asks of every finder test. `steering_claim` is the
   module of User Story 9, cut from User Story 3: the approval's own rule, one `mod` line
   for each new finder test module, applied to one more. No other line of those files
   and no other existing test changes. 040's flip approves its own modules under this
   number.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `015-tracking`, `030-imaging-camera-link`,
   `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
   `033-focus-reading-and-aid`, `034-dark-and-flat-frames`, `035-live-stack` and
   `038-track-follows-a-push`, all in this spec's `depends_on_landed`. Met: each landed
   with its last story's merge on `main`: 015 at `a02dc35b` (US9, #177), 030 at
   `49f8dcd2` (US8, #269), 031 at `e9687ce9` (US9, #279), 032 at `670ddcf9` (US7, #288),
   033 at `80f414ff` (US7, #298), 034 at `002dbef8` (US8, #309), 035 at `52785403`
   (US10, #321) and 038 at `1c2b427b` (US3, #260). 033 is named because it writes
   `imaging/mod.rs`, `imaging/routes.rs`, `lib.rs`, the README and the Imaging page
   before this spec does, and 035 depends on it; nothing of 033's is called here.
2. **The trio is re-anchored against `main` after they land.** Met on 2026-11-20, on
   `main` at `52785403`. Every `path:line` the trio cites, read at 49bb210, was read
   again there by its content and its number put right: 038 had moved most citations of
   `crates/finder/src/tracking.rs` from line 83 on (D7), so that the validator refused
   the citation of `impl Drop for Tracker`, and 030 to 035 had moved the others, among
   them the last route line of `crates/finder/src/lib.rs` and the last `/api/imaging`
   row of the README, after which US5 inserts its own and which § The route also names
   by content (D36). Every name the trio takes from 030 to 035 was read in their code:
   030's `imaging_truth_at`, `imaging_rotation_at` and `SimImagingCamera`; 031's
   `Imaging`, `Imaging::start`, `Imaging::view`, `ImagingView`, `CameraView`,
   `SettingsView`, `RoiView`, `SessionWrite`, `sim_json`, `put_settings`, `SettingsEdit`
   and `with_imaging`; 032's `ImagingSolve`, `from_cd`, `pixel_of`, `cd_deg`,
   `SolveFrame`, `Region` and the solving rig; 034's match of a master dark to a sub's
   exposure within 1 ms; and 035's, in § What 035 must provide. Each is as the trio
   names it. Where the trio took one from their plan.md, it now cites their code by
   line; elsewhere a file they created is named without a line, as 035's re-anchoring
   left its own. The other names this list held before the split (`set`, `FrameView`,
   `Refusal`, `putImagingSettings`, `set_unresponsive`, the camera's lease, `set_solver`,
   `solve_frame` and `SimulatedSolver`) and the `dl` of `TrackControl.tsx` are used by
   040's stories alone, which read them at 040's flip. The story that last wrote each
   shared file was read with `git log` on `main` and matches § Refinement Record, and
   the validator was re-run there.
3. **035 is read as landed, against § What 035 must provide.** Met on 2026-11-20 for
   the two points this spec's own stories use, each read in 035's landed code on `main`
   at `52785403`: the worker's settings test, which still drops a sub whose exposure
   differs from the reference sub's by more than 1 ms, so that owner decision 2's last
   alternative is still not possible; and the worker's queue of four waiting subs,
   which the re-anchoring added to the section. The rest of it, its nine points but
   for point 6's settings test, and its table of what changes in 035's two files,
   serve 040's US7, US8 and US11, and the re-anchoring took them out of this trio: 040's
   plan.md § What 035 provides holds them, and 040's flip.md § Prerequisites 3 lists
   what is settled of them before 040's flip. They were read on `main` at `52785403`
   too (history.md, the re-anchor of 2026-11-20).
4. **The owner decisions are settled and the approvals given** (§ Owner decisions and
   § Owner approvals above). Not met: this spec's decisions, 1 to 8 and 13 to 16, are
   open.
5. **A probe build.** Met on 2026-11-19 for the trio as round 1 found it, and not met
   for the trio as the reanalysis of the same day left it: nothing the two passes of
   that second fold wrote has been built (the end of this point says what a second round
   must show). Probe round 1 built all eight stories on 2026-11-18 and 19, tests first
   and with both gates, on probe builds of 030 to 035
   (spec.md § Clarifications, the session of round 1 probes). Before it, the bounds of
   US3-S1, US3-S2, US4-S5 and 040's US7-S1 came from arithmetic on a model of the loop
   (§ Refinement Record), and 040's US6-S5's rested on 015's own follow staying within 60″
   with the finder camera's corrections held for gaps 6 s apart before and after the
   measurement. US3 and 040's US6 were probe-built together, since 040's US6 changes the test
   `serve_gaps` step 1 makes and the held mode's test in `Shared::pointing`. What the
   round settled:
   - US3's test file passed before and after 040's US6, unedited (§ Plan traps 22).
   - Both rigs of 040's US7 ran. The stacking rig tracks and stacks on top of 035, 18 subs in
     about 97 s of virtual time, once its drift oracle reads each saved sub from its
     FITS file and it waits 1 s more before the stack starts (D37 and D38; spec.md
     § Clarifications, the session of round 1 probes); 040's US7-S5's scripted tracker, given
     with `set_source`, saw its link dropped within 1 s, well inside its 10 s gap.
   - The nine points of § What 035 must provide hold in 035's probe code, with the three
     words 040's plan.md § The stack and 040's plan.md § The page now give: the taker's receiver of the stopping
     mark is made `mut` (point 2); the `not_steady` drop is counted inline, as the
     settings drop is, and `LastView` does not carry it (point 7: true of that probe
     code and not of the 035 that landed, where the drop is answered from `process` and
     `Dropped::count` gets an arm for it, 040's plan.md § The stack); and the Stack block's
     filter reads an optional count with `?? 0` (point 8).
   - Every story's diff is under 64 KiB (§ Risks).
   - Every bound held on three runs each, with the same numbers each time. US3-S1: at
     most 0.0220″ from the 12th sub with each error handed over at once, and 0.0340″
     played late, against 0.15″ (4.4× at the late sub; the model's 0.030″ was 13% under
     it).
   - US3-S2: a median drift of 17.19″ without feedback, against "above 3.0″" (5.7×).
   - US4-S5: at most 0.01552″ from the 14th sub, against 0.15″ (9.7×).
   - 040's US6-S5: a worst true error of 41.414″, against 015's 60″ (1.45×), with the play
     measured at 0.299879°. With the held mode's test left at `lapsed` alone, 015's
     bound broke at transit + 319.7 s, as 040's plan.md § Plan traps 28 says it would.
   - 040's US7-S1: at most 0.01558″ from the 14th sub, against 0.15″ (9.6×; 0.0317″ at the
     12th), and 0.746″ on every sub with the switch off.

   **What the reanalysis settled, and what a second probe round must show.** Six reviews
   of the folded trio found 78 more issues, and the owner took the recommended answer to
   each of the fifteen choices they came to (spec.md § Clarifications, the session of
   the reanalysis). The re-run among them built round 1's 60 folded cases on all eight
   stories: 59 passed on three runs each, with the five bounds unchanged to the digit,
   and the sixth lacked only D25's code fix, which that probe never had. It also ran two
   of the fixes now in the trio, a lapsed gap that always sends each axis its rate, with
   the whole workspace passing, and an error from before a follow dropped by its time.
   Everything else of the second fold rests on reading and on models of this plan's law:
   two of them reproduce round 1's two measured drifts and give, with the rate's limit a
   gap and the two-deadband rule added, the same 12th-sub figures to four places. Probe
   round 2 builds this spec's six stories on `main`, with 035 landed, and must show:
   - every bound of round 1 again under the changed law: US3-S1 and US3-S2 with the
     rig's 3.5 s, and US4-S5 from the 14th sub;
   - the scenarios nobody has run: US5-S11's hour ahead;
   - that every test file passes unedited under every later story, and US9's on the code
     of US3, the story it was cut from, with no test of its failing first;
   - each story's diff against its estimate in § Risks, US3 and US5 above all;
   - the points of flip.md § Prerequisites 3 that 035's US9 and US10 settle, now that they have
     landed.
   What round 2 must show of 040's stories is in 040's flip.md § Prerequisites 5.
6. **Outside the factory, before this spec is used at the telescope:** GoTo and Track
   proven on the real mount (the plan document, § Before any flip, 4), and its backlash
   measured on both axes: altitude reverses at transit, and azimuth too for a target
   that culminates north of the zenith. Not met.
7. **The documents this trio cites are on `main`.** Met: the plan document,
   ADR-0002, the changed `GLOSSARY.md` and the research note have been on `main`
   since 030's flip (`55e41f57`). US2 inserts one of its entries after **Imaging
   angle**, which only the rewrite has (`GLOSSARY.md:107` at `52785403`).
8. **No operator-staged files are needed.** Met.
9. **The plan document agrees with the settled decisions.** Not met. `main` has held
   the plan document since `55e41f57` (§ Prerequisites 7), and on `main` at `52785403`
   `docs/plans/2026-11-15-specs-030-037-imaging.md` § Interfaces shared across the specs
   carries this trio's recommended answers: its route table has the row
   `PUT /api/imaging/steering` for 036 (owner decision 3); its list of the parts of
   the session's `imaging` object names `steering`, and its module tree
   `crates/finder/src/imaging/steering.rs` (owner decision 14); and its list of MCP
   tools has none for 036, whose `steering` part is read through `get_imaging_camera`
   and the session tools, beside the "Who runs it" row's "Each step also has its own
   route and MCP tool" (owner decision 11). That last answer changed on 2026-11-19: 036
   has one tool, `set_steering`, and the plan document's list gains it in the same
   change as the flip; it is not edited by this fold. Its § Simulator names what US1
   adds, a memory of where the telescope pointed and count steps, where its first draft
   had "rate error within an exposure (036)" (spec.md § Clarifications). If the owner
   settles any of those decisions otherwise, the plan document is corrected in the same
   change: the route's row, the part, the module, and the tool or the exemption. Rule 1
   is not edited: it gives its two times a tracker is not steady as today's examples,
   and owner decision 17 declares the three further times on its own, and since
   2026-11-19 three more. 035's Prerequisites 10 does the same for its own departures.
10. **031 carries the camera's lease.** Met: 031's US9 landed it on `main`
    (`e9687ce9`), with `Imaging::lease`, `Lease::take` and `Lease::set`
    (`crates/finder/src/imaging/mod.rs:711`, `crates/finder/src/imaging/mod.rs:884`,
    `crates/finder/src/imaging/mod.rs:876`), and 035 holds it for a stack. Since the
    re-anchoring of 2026-11-20 this trio does not reach the lease: 040's stories do
    (040's plan.md § What 035 provides).
11. **A bound on the rate one tracking error may ask for.** Met on 2026-11-17, with
    nothing to add here. 032's owner decision on a newly measured aim point while Track
    is on (032's owner decision 25, raised by the audit's finding A4 on 032; 037's
    owner decision 26 cites it) could have taken the answer that the tracker bounds the
    rate one tracking error may ask for, which US3 would then have carried. The owner
    chose the other answer: 032 holds a newly measured aim point back while Track is on
    and uses it once Track is off (032's FR-036). So no bound is written, and nothing in
    this trio changes.
12. **038's followed push.** Added 2026-11-16; met on 2026-11-19.
    `038-track-follows-a-push` makes Track follow a push by hand of more than 5′ through
    `Tracker::follow`, which changes the place the tracker holds. 038 has landed
    (`ad09efeb`), so this trio is re-anchored to it: `follow` ends any claim the subs
    hold on the tracker and forgets any tracking error or held correction not yet
    applied, as `release` does (spec FR-011; § The tracker, **`follow`**), with one
    scenario in US3, US3-S12 [now US9-S12]; `038-track-follows-a-push` joins
    `depends_on_landed` and US3's shared write of the tracker; and the validator is
    re-run. Without that, a sub's error, measured from the old place, would steer the
    tracker back toward it at up to 30″ a gap. The reanalysis completed it on
    2026-11-19: `follow` also drops, by its time, an error from the old place that is
    handed over after the claim was taken
    again; it forgets what a lapsed claim held, with a scenario of its own (US9-S13);
    and the stretch before the follow, while 038 reads the push as moving, is a time the
    tracker is not steady (§ The tracker's two answers, **A push by hand**; 040's US6-S6).

## Field checks, outside the factory

Before this spec is used at the telescope (flip.md § Prerequisites 6):

- drift per sub with the switch on and off, on a bright field;
- drift per sub while subs are being dropped, the finder camera correcting in the gaps;
- whether a half-second pulse of one count moves the tube, and the tube's motion per
  commanded pulse of 1″, 10″ and 30″;
- how late a tracking error arrives with the real solver at the camera's full
  resolution, and the worker's time for one sub (035's `took_ms`) against the sub
  cadence;
- the two shares of owner decision 4 confirmed or retuned on the night with the tuning
  settings (FR-030) and then written into the code, with the constants of owner decision
  5, which only a rebuild changes;
- rebuilding the console on the console machine.

The checks of 040's stories (a run through the meridian, the scope node's journal, the
axis counts, the first reversal's measurement and the Impeccable review) are in 040's
flip.md.

## The prerequisites as tasks.md listed them

Moved here from tasks.md, which now points to flip.md § Prerequisites above.

**Prerequisites** (flip.md § Prerequisites):
- `015-tracking` and `038-track-follows-a-push` have landed on `main`: met;
- `030-imaging-camera-link`, `031-imaging-frames-on-console`, `032-imaging-solve-and-aim-point`,
  `033-focus-reading-and-aid`, `034-dark-and-flat-frames` and `035-live-stack` have
  landed, 031 with the camera's lease: not met;
- this trio has been re-anchored against `main` after they landed, each point of
  plan.md § What 035 must provide read in the landed code, and 040's US7's calls placed
  against 035's taker, its returns and its worker by name and line: not met;
- the owner decisions in spec.md are settled and its owner approvals given, and the
  plan document agrees with them (flip.md § Prerequisites 9; in this worktree it
  carries the recommended answers, and it is not on `main`): not met;
- a probe build has confirmed the bounds of US3-S1, US3-S2, US4-S5, 040's US6-S5 and 040's US7-S1,
  and that US3's test file passes unedited once 040's US6 has landed: met on 2026-11-19 by
  probe round 1 for the trio as it then stood, and not met for the trio as the
  reanalysis of that day left it, which no build has run: a second probe round builds
  all eleven stories (flip.md § Prerequisites 5);
- the documents this trio cites are on `main`: not met;
- 032's owner decision on a newly measured aim point while Track is on is settled,
  and, if its answer is that the tracker bounds the rate one tracking error may ask
  for, US3 carries that bound with one scenario (flip.md § Prerequisites 11): met on
  2026-11-17; the owner chose to hold the aim point, so no bound is added;
- 038's `follow` ends any claim the subs hold and forgets what was pending or held, with
  one scenario in US3 (flip.md § Prerequisites 12): met on 2026-11-19, as the scenario
  that is now US9-S12, and completed by the reanalysis with US9-S13 and 040's US6-S6;
- no operator-staged file is needed.
