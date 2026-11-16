---
state: draft
depends_on_landed: ["014-goto", "015-tracking", "016-mcp", "017-console-stop-confirm", "024-mount-link-recovery", "025-frame-capture", "027-finder-tests-by-system", "028-finder-tests-do-less"]
# Drafted 2026-11-16 against `main` at 49bb210 (49bb210fe4860dd98ba24727074e7bd3cc0d7124),
# at the owner's ask the same day: "If I push it away from the Orion nebula to look at
# something near it. I don't want it to try to point back to the Orion nebula without me
# telling it too. Perhaps instead, it could just show that it has drifted from the target
# but maintain tracking". He chose a spec of its own ("Yes, draft 038") over folding it
# into 037. It changes 015's Track, which has landed, and lands on its own: nothing in
# 030 to 037 is needed for it. 037 gains one ending from it.
# A second agent reviewed the first draft the same day against the code; its nine
# findings are answered in this text (Clarifications, last session).
# Not yet probe-built: no story of this trio has been compiled or run (plan.md
# § Refinement Record). Every `path:line` here was read at 49bb210.
# It stays draft until the owner settles the owner decisions and flips it to `ready`,
# which gives the approvals listed under "Owner approvals" and dispatches it.
---

# Feature Specification: Track Follows a Push

**Feature Branch**: `038-track-follows-a-push`

**Created**: 2026-11-16

**Status**: Draft

**Input**: Today Track holds the session's target at the aim point. If the observer
pushes the telescope by hand while Track is on, the next live pointing shows the push
as a tracking error, and 015's correction works the telescope back to the target. The
owner wants the opposite for a deliberate push. A push of more than a few arcminutes
moves Track: once the telescope is left still, the console keeps tracking the patch of
sky it now points at, shows how far that is from the target, and goes back only when
the observer asks. A small disturbance, such as a knock, is still corrected as today.

## Clarifications

### Session 2026-11-16 (the owner's ask, and a review of `main` at 49bb210)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What does Track do today after a push? → A: Five facts, each read at 49bb210:
  1. The scope node turns the mount's own hand-push encoders off when it connects
     (`crates/scope-node/README.md`, "`:W sent`"), so a push changes no axis count. The
     console sees a push only in the finder camera's next live pointing.
  2. Each live pointing reaches the tracker through `Session::track_live_pointing`
     (`crates/finder/src/session.rs:972`), called by the live loop before the same
     frame is offered as a sync (`crates/finder/src/live.rs:216`,
     `crates/finder/src/live.rs:223`). It is handed over as the aim point's error
     from the tracker's target.
  3. The tracker turns every error of 10″ or more into a proportional rate, −0.6 of
     the error over the interval since the last pointing, with no bound on its size
     (`crates/finder/src/tracking.rs:304`). A rate against the axis's tracking
     direction is sent as zero instead (`crates/finder/src/tracking.rs:873`). So a
     push "behind" the target, against the sky's motion, is driven back at once, and a
     push "ahead" of it stops that axis until the sky brings the target back. Either
     way the telescope ends on the target.
  4. Turning Track on while it already tracks the same target is answered at once and
     changes nothing (`crates/finder/src/session.rs:825`), so the observer has no way
     to send the telescope back except Track off and on.
  5. The pointing model calls a sync a slip when it lies more than 0.5° from the
     model's prediction once a base tilt is fitted, and 2.5° before
     (`crates/finder/src/model.rs:497`). A smaller push is fitted into the night's zero
     points as if the axes had not moved.
- Q: Where does the push check live? → A: In the session, in the live-pointing intake,
  before the tracker is handed the error. The tracker's own `pointing` keeps its
  meaning: aim point minus target, corrected as 015 has it. An existing test feeds it
  errors of 10,000″ on purpose to clamp the tracking offset
  (`crates/finder/tests/motion/tracking/pi_loop.rs:412`), and that test stays as it is
  (plan.md § Shapes that existing tests pin).
- Q: What counts as a push? → A: A live pointing more than the push threshold, 5′ by
  default, from the place Track holds (owner decision 1). From that pointing the
  telescope is **moving by hand**: the session hands the tracker nothing, so the
  tracker keeps its rates and its learned offset and pulls nowhere. The push has
  **settled** once the live pointings have stayed within 1′ of one another for 5 s,
  and Track then **follows** the latest one (owner decision 2). A pointing back within
  the threshold of the place held, before the push settles, ends the moving state: the
  observer put the telescope back, and it is corrected as today.
- Q: What if the observer keeps pushing after a pause? → A: For 10 s after a follow, a
  pointing more than 1′ from the place followed makes the telescope moving by hand
  again. After that the threshold applies as before (owner decision 2).
- Q: What does following mean? → A: The tracker keeps running. It takes the settled
  pointing's J2000 place as the place it holds, and from its next turn computes its
  rates, its reversal forecasts, its keyhole plan and its altitude guard for that place.
  The session's target does not change. The page shows it as today, and its guidance
  block already gives the turn and tilt from the latest pointing to it; an MCP client
  also sees the place followed in the session.
- Q: How does the observer go back? → A: By turning Track on for the target again: the
  page's "Recenter {target}" button, `POST /api/track` with `on` true, or the MCP tool
  `set_tracking`. While Track follows a push or the telescope moves by hand, that is no
  longer the no-op of fact 4: it ends the tracker and does what turning Track on does
  for any start, a GoTo first when the pointing is more than 1′ from the target, then
  tracking (015's FR-014; owner decision 6).
- Q: When does the check not run? → A: Until it is armed, and while a backlash is
  measured. It is armed by the first live pointing at or under the threshold handed to
  the tracker since the tracker started, since it left the zenith keyhole and since the
  aim point last changed. So a model error after the keyhole, or an aim point moved by
  a tap, is corrected as today and never read as a push. While an axis's backlash is
  being measured, 015 lets the error grow on purpose
  (`crates/finder/src/tracking.rs:646`); the simulated mount's backlash is 0.3°, more
  than the threshold (owner decision 4).
- Q: What happens to the pointing model? → A: A push is a slip by the glossary's own
  words: the axes moved without their readings following. When the telescope starts
  moving by hand, the session marks the model's next sync as a slip. While it moves,
  frames are offered as no sync. The first sync after that opens a new zero-point epoch
  whatever its distance from the model's prediction, and if Track is following, the
  tracker is handed the session's model then, so its keyhole plan is made with the new
  zero points (owner decision 3).
- Q: Does anything here know about alt-az mounts? → A: No. The check, the settle and
  the distance are separations on the sky in J2000. An equatorial mount pushed by hand
  would be handled the same way.
- Q: What about an imaging run? → A: 037 ends its run `pushed_away` when the session's
  Track view shows a push, and leaves Track on. That is 037's text, changed there; this
  spec only gives it the `push` part of the Track view to read.
- Q: Does the page say anything new about the push? → A: No. The owner, the same day:
  "the webpage already shows what the object logged in goto is, we dont need any extra
  details like that we just need some sort of recenter target functionality to use for
  visual observing." The page's only addition is a "Recenter {target}" button while a
  push is shown (FR-012, FR-013).
- Q: What is left for later? → A: The items under "Out of scope" in § Assumptions.

### Session 2026-11-16 (a second agent's review of the first draft)

The first draft had a 10′ threshold, settled on two pointings in a row, turned the
check off during take-ups, and kept the push state on the Track run alone. The review
read it against the code and found, among others, each of these; the answer is in the
text above or below:

- the push state outlived a Track that ended by itself, and a moving state then
  refused every later sync for good: it is now read as none whenever the tracker is off,
  and cleared by Track off (FR-009, plan.md § The session);
- M43 lies about 7.5′ from M42, under a 10′ threshold: the threshold is now 5′, and
  owner decision 1 names it;
- two pointings in a row settle in the middle of a slow push: the settle is now 5 s of
  stillness, with a 10 s window after a follow;
- the scenarios pushed "ahead" of a rising target, where today's code does not pull
  back quickly either, so they proved little: every scenario now pushes down, behind
  M42, where today's code would pull back at once;
- a keyhole exit or an aim change could be read as a push: the check is now armed only
  after a pointing within the threshold;
- the slip distance is 2.5° before a base tilt is fitted, as this rig has: fact 5 says
  so, and US1-S5 now proves the new epoch below it;
- `follow` could race the keyhole: it is now refused unless the tracker is tracking and
  measuring nothing, and the session rechecks Track's generation before writing.

## Owner decisions

Each is a design or taste call. None is settled. A different answer is written into
the trio and re-validated before the flip, never left to a story.

1. **The push threshold.** Recommended: 5′, one constant. M43, 7.5′ from M42, is then a
   push; a knock that moves the eyepiece less is still corrected, and tracking holds
   well inside it (the deadband is 10″). Instead: 10′, which pulls a push to M43 back;
   or 2.5′, which follows more knocks.
2. **When a push has settled.** Recommended: 5 s with every pointing within 1′ of the
   first of them, and for 10 s after a follow a move of more than 1′ counts as moving
   again. Instead: two pointings in a row within 1′, which settles mid-push on a slow
   nudge; or 10 s.
3. **The pointing model after a push.** Recommended: a slip is marked when the telescope
   starts moving by hand, frames taken while it moves are no sync, and the next sync
   opens a new zero-point epoch. Instead: leave it to the model's own slip check, as
   today.
4. **When the check runs.** Recommended: once armed by a pointing within the threshold,
   and not while a backlash is measured. Instead: always, which would read a keyhole
   exit or the simulator's own backlash as a push.
5. **The tracker while moving by hand.** Recommended: it keeps its feed-forward rates
   and its held tracking offset and makes no correction. Instead: stop both axes until
   the push settles.
6. **Going back.** Recommended: turning Track on for the target, from the page's new
   "Recenter {target}" button, the route or `set_tracking`. Instead: only a GoTo, which leaves Track to be
   turned on again.
7. **The learned tracking offset.** Recommended: kept across a follow, since it is the
   mount's own rate error and does not depend on where it points. Instead: cleared.
8. **The readout.** Recommended: while Track follows a push, the field rotation and the
   longest exposure are computed at the place followed, and the RMS error is over the
   pointings since the follow. Instead: keep them at the target.
9. **No new MCP tool.** Recommended: an MCP client reads `track.push` through the
   session tools and goes back with `set_tracking`, whose description gains one
   sentence. Instead: a `back_to_target` tool.

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test, golden or CI file changes, but for the three `mod` lines below.

1. **Track no longer always holds the target.** After a followed push it holds another
   place until the observer goes back. This changes the meaning of 015's FR-006 and
   FR-008 (FR-003, FR-004).
2. **Turning Track on while a push is shown sends the telescope back**, where today it
   is a no-op (FR-008).
3. **A new optional key in the session's `track` view, `push`**, and the readout
   computed at the place followed (FR-009, FR-010).
4. **Two changes to 014's pointing model:** frames taken while moving by hand are no
   sync, and a pending slip is kept in the state file as `slip_pending`, written only
   while true (FR-006, FR-007).
5. **The page's "Recenter {target}" button,** recorded in `DESIGN.md` and
   `.impeccable/design.json` (FR-012 to FR-014).
6. **A glossary term, *Followed push*** (FR-015).
7. **One sentence added to `set_tracking`'s description** (FR-011).
8. **Three `mod` lines**, each in alphabetical order in a file that holds no test:
   `mod push;` (US1) and `mod push_back;` (US2) in
   `crates/finder/tests/motion/tracking/mod.rs`, and `mod push_tool;` (US2) in
   `crates/finder/tests/mcp/main.rs`.

## User Scenarios & Testing *(mandatory)*

Every Rust scenario shares these definitions.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command. The page's scenarios run in `npm test` in `crates/finder/web`.
- **The rig** is `crate::support::goto::Rig` (`crates/finder/tests/support/goto.rs:104`)
  built with `Rig::new(1_768_528_800.0, 50.0, 120.0, true)` under
  `#[tokio::test(start_paused = true)]`: the simulator at 40° N 75° W, its mount
  connected, finder frames every 500 ms (`crates/finder/tests/support/goto.rs:146`)
  solved by the rig's `Stars` solver, a live loop and a sync. Its model has no base
  tilt. **On M42** means the rig after `rig.sim.point_eyepiece_at` on M42's catalogue
  place, a sync, and `POST /api/track` with `{"on":true}` until `track.state` reads
  `tracking`, as `centred()` does in `crates/finder/tests/motion/tracking/routes.rs:95`.
  M42 is then near 43° altitude, rising about 3.6″ a second, 53 minutes before transit;
  no axis reverses in any scenario's window.
- **A push** is `rig.mount.push_by(turn, tilt)` in degrees
  (`crates/simulator/src/mount.rs:269`): the tube moves, no axis count changes. Every
  scenario pushes down in tilt, behind the rising target, which moves the eyepiece by
  the same angle on the sky and is the direction today's code pulls back at once.
- **The true place** at a moment is the J2000 direction of the simulator's eyepiece
  then, `equatorial(alt, az, t)` of `rig.mount.true_direction()` as
  `crates/finder/tests/support/tracker.rs` computes it. **Its distance** from a place is
  `finder::model::separation` in degrees.
- **Waiting until** a condition means reading `rig.view()` every 100 ms of virtual time
  until it holds, for at most 120 s, and failing otherwise.
- **Oracles.** The simulator's true place, the session's view, and the pointing model's
  epochs and syncs as `rig.state.session().model(t)` reads them. A count or a word has
  no tolerance; every other tolerance is stated where the number is.

### User Story 1 - A push of more than 5′ is followed, a smaller one is corrected (Priority: P1)

As the observer, I push the telescope by hand from M42 to something nearby while Track is
on. The console does not drag it back. Once I let go, it tracks where I left it.

**Why this priority**: It is the owner's ask, and every later story reads its state.

**Independent Test**: `crates/finder/tests/motion/tracking/push.rs`, the module
`tracking::push` of finder's `motion` test binary. During development,
`cargo nextest run -p finder --locked --test motion tracking::push::`.

**Acceptance Scenarios**:

1. **Given** the rig on M42, **When** a committed test pushes by (0.0, −2.0) and waits
   until `track.push.state` is `following`, **Then** it asserts that:
   - `track.state` is `tracking`, `track.wanted` is true and `track.target.id` is still
     `NGC 1976`;
   - `track.push.at` is at least 5 s after the push;
   - `track.push.ra_deg` and `track.push.dec_deg` lie within 60″ of the true place at
     the time of the view;
   - sampled every second for the next 120 s, the true place stays within 60″ of
     `track.push`'s place and more than 1.9° from M42.
2. **Given** the rig on M42, **When** a committed test pushes by (0.0, −0.05), 3′ on the
   sky, **Then** it asserts that within 30 s the true place is within 60″ of M42, that
   no view in those 30 s has a `push` key, and that `track.state` is `tracking`
   throughout.
3. **Given** the rig on M42, **When** a committed test pushes by (0.0, −1.0), waits 1 s
   of virtual time, pushes by (0.0, −1.0) again and waits until `track.push.state` is
   `following`, **Then** it asserts that:
   - every view between the first push and the follow had `track.push.state` `moving`
     and no place;
   - the true place's distance from M42 never fell by more than 1′ between two views in
     that time;
   - `track.push.at` is at least 5 s after the second push, and the place followed is
     within 60″ of the true place then.
4. **Given** the rig on M42, pushed by (0.0, −1.0) until a view shows `track.push.state`
   `moving`, **When** a committed test posts `POST /api/track` with `{"on":false}`,
   **Then** it asserts that the answer's `track.state` is `off`, `track.ended_reason`
   `stopped`, that `track` has no `push` key, and that within 10 s the model has a sync
   timed after the Track off: a moving state does not outlive Track.
5. **Given** the rig on M42, whose model has no base tilt and so a slip distance of
   2.5°, **When** a committed test notes the model's epochs and the push's time, pushes
   by (0.0, −0.25), 15′, and waits until `track.push.state` is `following` and the
   model has a sync timed after the push, **Then** it asserts that the model has
   exactly one more epoch than before the push, that every sync of an earlier epoch is
   timed before the push, and that every sync of the newest epoch is timed after it.
6. **Given** the rig on M42 with the minimum altitude set 1° under M42's altitude,
   **When** a committed test pushes by (0.0, −3.0) and waits until `track.state` is
   `off`, **Then** it asserts that `track.ended_reason` is `too_low`, that `track` has
   no `push` key, and that both axes are at rest within 2 s of that view.
7. **Given** a tracker on M81 built as `crates/finder/tests/motion/tracking/backlash.rs:9`
   builds one, 600 s before its first azimuth reversal, **When** a committed test reads
   `tracker.state().checks_push` every second while that rig's own `follow` helper feeds
   it pointings to 600 s past the reversal, **Then** it asserts that some reading is false while the session has
   no `backlash.az_deg`, and that the last reading, after `backlash.az_deg` is stored,
   is true.
8. **Given** the rig on M42, **When** a committed test moves the aim point with
   `Session::set_aim` 40 pixels to the right of the aim in force, about 20′ on the sky
   at the finder camera's 29.78″ a pixel, **Then** it asserts that no view in the next
   60 s has a `push` key and that `track.state` stays `tracking`: an aim change disarms
   the check, and the tracker corrects it as today.

---

### User Story 2 - Going back to the target (Priority: P1)

As the observer, having looked around, I ask for M42 again. The telescope goes back and
tracks it.

**Why this priority**: Without it a followed push could be undone only by Track off and
on, which the owner did not ask for.

**Independent Test**: `crates/finder/tests/motion/tracking/push_back.rs`, the module
`tracking::push_back` of the same binary, a new file: US1's `push.rs` is an existing
test by then.

**Acceptance Scenarios**:

1. **Given** the rig following a push as in US1 scenario 1, **When** a committed test
   posts `POST /api/track` with `{"on":true}`, **Then** it asserts that the answer is 200
   with `track.state` `armed` and no `push` key, that `goto.state` is `slewing`,
   `solving` or `correcting`, and that it then waits until `track.state` is `tracking`,
   after which the true place is within 60″ of M42.
2. **Given** the rig on M42, **When** a committed test pushes by (0.0, −1.0), waits until
   a view shows `track.push.state` `moving`, and at once posts `POST /api/track` with
   `{"on":true}`, **Then** it asserts the same as scenario 1.
3. **Given** the rig on M42 with no push, **When** a committed test posts
   `POST /api/track` with `{"on":true}`, **Then** it asserts that no GoTo starts and
   that `track.state` stays `tracking`, as today.
4. **Given** an MCP app built from `crate::support::mcp::state()` and `app(state,
   "127.0.0.1")` (`crates/finder/tests/support/mcp.rs:24`), **When** a committed test
   in the new module `crates/finder/tests/mcp/push_tool.rs` sends `tools/list`,
   **Then** it asserts that `set_tracking`'s description contains `real telescope` and,
   exactly, the sentence of FR-011.

---

### User Story 3 - A Recenter button for visual observing (Priority: P2)

As the observer at the eyepiece, having pushed the telescope to look around, I press one
button to bring M42 back to the centre of the field.

**Why this priority**: The routes carry everything; this is the button at the
telescope.

**Independent Test**: `crates/finder/web/src/components/TrackPush.test.tsx`, a new
Vitest file run by `npm test`.

**Acceptance Scenarios**:

1. **Given** a session whose `track` is tracking M42 with `push`
   `{"state":"following","ra_deg":84.0,"dec_deg":-3.4,"at":1768529000.0}`, **When** a
   committed test renders `TrackControl`, **Then** it asserts that a button named
   `Recenter M42` is shown, and that the status reads exactly as it does for the same
   session with no `push`.
2. **Given** the same session with `push` `{"state":"moving"}`, **When** a committed test
   renders `TrackControl`, **Then** it asserts that the same button is shown.
3. **Given** scenario 1's session, **When** a committed test presses `Recenter M42`,
   **Then** it asserts that one `POST /api/track` was sent with `{"on":true}`, that the
   switch was not touched, and that a refusal's error is shown as an alert.
4. **Given** a session with no `push`, **When** a committed test renders `TrackControl`,
   **Then** no `Recenter` button is shown and the block is as today.

---

### Edge Cases

- **A push during GoTo, or while Track is armed.** No tracker is running, so no check
  runs. GoTo's own corrections handle it, as today.
- **A push before the check is armed, or during a backlash measurement.** It is
  corrected as today. The check arms within a pointing or two of Track starting.
- **A push at the keyhole.** The tracker ignores pointings there already; the check is
  armed again only by a pointing within the threshold after the exit.
- **A push that never settles**, because the observer keeps moving the tube or the frames
  do not solve. The telescope stays moving by hand; the tracker keeps its rates for the
  old place and pulls nowhere. Its guards still judge the old place until a follow.
- **A push put back by hand** within 5′ of the place held, before it settles, ends the
  moving state and is corrected. The slip mark stays: the next sync opens a new epoch,
  which costs the night's earlier syncs and nothing else.
- **A push back onto the target by hand after a follow.** It is more than 5′ from the
  place followed, so it settles and is followed. The status then reads, say,
  `6′ from M42`; going back stays available.
- **A follow below the minimum altitude, or into the keyhole.** The tracker's guard ends
  Track as `too_low`, or its keyhole step runs for the new place, from its next turn.
- **Track off, Stop, a lost link, a new target, or any ending of the tracker** end the
  push state with it: the view drops `push` and the sync rule lets syncs in again.
- **A newly measured aim point while Track is on** (032, if its owner decision 25 is
  answered "use it at once") moves every live pointing by the change. A change of an
  aim point disarms the check (FR-001), so it is corrected as today, which is the
  hazard 032's decision is about.

## Requirements *(mandatory)*

### Functional Requirements

**The check** (US1):

- **FR-001**: For each live pointing handed to `Session::track_live_pointing` while the
  tracker's `checks_push` is true and the check is armed, the session SHALL compute the
  pointing's separation from the place the tracker holds. The check SHALL be armed by
  the first pointing at or under the push threshold, 5′ (owner decision 1), handed to
  the tracker since the tracker started, since it last left the keyhole and since the
  aim point last changed (owner decision 4). At or under the threshold, the session
  SHALL hand the tracker the error as today.
- **FR-002**: Over the threshold, the session SHALL hand the tracker nothing and SHALL
  mark Track as moving by hand. While moving, it SHALL keep the first pointing of the
  current still stretch and its time; a pointing more than 1′ from it SHALL start a new
  stretch. When a stretch has lasted 5 s, the push has settled (owner decision 2). A
  pointing at or under the threshold from the place held SHALL end the moving state and
  be handed over as in FR-001.
- **FR-003**: When a push settles, the session SHALL call `Tracker::follow` with the
  latest pointing's J2000 place, and Track SHALL follow that place until it ends or is
  turned on again for the target. For 10 s after a follow, a pointing more than 1′ from
  the place followed SHALL make Track moving by hand again, with that pointing starting
  the stretch.
- **FR-004**: `Tracker::follow` SHALL make the place it is given the tracker's target for
  every later pointing, rate, reversal forecast, keyhole plan and guard, from the rate
  loop's next turn. It SHALL clear the proportional term, the previous pointing's time
  and the error samples, and SHALL keep the tracking offset (owner decision 7). It SHALL
  be refused, changing nothing, unless the tracker is `tracking` with no backlash being
  measured, checked under the tracker's own lock.
- **FR-005**: The tracker's state SHALL carry `checks_push`: true while it is `tracking`
  and no axis's backlash is being measured, false otherwise, including `off` and
  `keyhole_wait` (owner decision 4).

**The pointing model** (US1):

- **FR-006**: While Track is moving by hand and the tracker is running,
  `Session::record_frame_sync` SHALL record no sync and answer `Ok(false)`.
- **FR-007**: When Track first becomes moving by hand, the session SHALL mark the
  model's next sync as a slip. The next sync recorded SHALL open a new zero-point epoch
  whatever its distance from the model's prediction, and the mark SHALL then clear. The
  mark SHALL be kept in the state file as `slip_pending`, written only while true. When
  that sync is recorded while Track follows a push, the session SHALL hand the tracker
  its model, which the tracker SHALL use for its later plans (owner decision 3).

**Going back** (US2):

- **FR-008**: Turning Track on for the session's target while Track follows a push or is
  moving by hand SHALL end the tracker and clear the push state, then do what any Track
  start does under 015's FR-014: a GoTo first when the pointing is more than 1′ from the
  target, then tracking. Turning it on with no push state SHALL stay the no-op it is
  today (owner decision 6).

**The session** (US1):

- **FR-009**: The session's `track` view SHALL carry `push` while the tracker is running
  and Track is moving by hand, as `{"state":"moving"}`, or follows a push, as
  `{"state":"following","ra_deg","dec_deg","at"}`: the place followed in J2000 degrees
  and the console time of the follow. It SHALL leave the key out otherwise. A push state SHALL be
  read as none whenever the tracker is off, and SHALL be cleared by Track off, Stop, a
  new target and every Track start.
- **FR-010**: While Track follows a push, the readout's field rotation and longest
  exposure SHALL be computed at the place followed (owner decision 8).
- **FR-011**: The MCP tool `set_tracking`'s description SHALL gain the sentence
  `Turning Track on while it follows a push by hand sends the telescope back to the target.`
  and SHALL otherwise be unchanged (owner decision 9).

**The page** (US3):

- **FR-012**: While `track.push` is present, `TrackControl` SHALL offer a button
  `Recenter {target}`, the target named as the status names it today, that sends
  `POST /api/track` with `{"on":true}` and refreshes the session, showing a refusal as
  an alert, as the switch does.
- **FR-013**: `TrackControl`'s status and readout SHALL stay as they are, with no words
  about the push: the guidance block already names the target and gives the turn and
  tilt to it.
- **FR-014**: `DESIGN.md`'s "Track and tracking readout" section and
  `.impeccable/design.json`'s "Track and Tracking Readout" component SHALL record the
  button as built.

**Words** (US1; US3):

- **FR-015**: `GLOSSARY.md` SHALL define **Followed push**: "A push by hand of more than
  the push threshold while Track is on, after which Track holds the place the telescope
  was left at until the observer turns Track on for the target again."
- **FR-016**: `crates/finder/README.md` SHALL describe the push check, the `push` key and
  going back, in the Track section.

### Key Entities

- **Push state**: none, moving by hand (with the still stretch's first pointing and its
  time), or following (the place, the time). Held in the session's Track run with
  whether the check is armed, never persisted, read as none while the tracker is off.
- **Place followed**: a J2000 direction the tracker holds in place of the target.
- **Slip mark**: one boolean on the pointing model, kept in the state file while true.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After a push of 2° behind the target while Track is on, the telescope is
  never driven back toward it: its distance from the target falls by no more than 1′
  between two reads until the observer asks (US1 scenarios 1 and 3), where today's code
  would drive it back at once.
- **SC-002**: After a push of 3′, the target is back within 60″ of the aim point within
  30 s (US1 scenario 2).
- **SC-003**: One request takes the telescope back to the target and tracks it (US2).
- **SC-004**: A session read with no push is byte-for-byte what it is today, and every
  existing test passes unchanged.

## Assumptions

- The finder camera's live pointings keep coming during and after a push. While the
  tube moves fast, frames may smear and not solve; that only lengthens the moving state.
- A push of 5′ or more that stays put for 5 s is deliberate. A knock that large would be
  followed; going back is one press.
- Real-mount behaviour is unproven: GoTo and Track have not yet run on the 400P to a
  target. The field check is in plan.md § Prerequisites.
- **Out of scope:** pushes during GoTo; dragging the target's place on the page; a set
  point other than the aim point; a push check from imaging subs, which 036 would add if
  it wants one.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007, FR-009, FR-010, FR-015]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-008, FR-011]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-012, FR-013, FR-014, FR-016]
```

Every edge is a merge edge, and the stories run one after another.

- **US2** changes `Session::start_tracking` beside US1's push state in
  `crates/finder/src/session.rs`, copies the few rig helpers it needs from US1's
  `push.rs` into its own `push_back.rs`, and adds its `mod` line after US1's.
- **US3** reads the `push` key US1 adds and sends the request US2 gives meaning to; it
  writes the README after US2's MCP sentence so the two agree.
