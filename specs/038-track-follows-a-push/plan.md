# Implementation Plan: Track Follows a Push

**Branch**: `038-track-follows-a-push` | **Date**: 2026-11-16 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Put a push check in the session's live-pointing intake. Once armed by a pointing within
5′ of the place Track holds, a live pointing more than 5′ from it is not handed to the
tracker, so nothing pulls the telescope back, and the pointing model's next sync is
marked as a slip. When the pointings have stayed within 1′ of one another for 5 s, the
session tells the tracker to follow that place and shows the push in the session's
`track` view. Turning Track on for the target again while a push is shown
sends the telescope back through 015's own GoTo-then-track path. The page reads the
push and offers a "Recenter {target}" button; it adds no other words.

The tracker's `pointing` is untouched: it still means aim point minus target and is
corrected as 015 has it. Nothing here changes GoTo, the PI loop's gains, backlash or the
keyhole procedure.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI. TypeScript with React 19 for the
page.

**Primary Dependencies**: nothing new.

**Storage**: one optional field in the state file, the model's `slip_pending`, written
only while true. Track and its push state are never persisted, as today.

**Testing**:
- Rust: new modules of existing test binaries. `tracking::push` and
  `tracking::push_back` in `crates/finder/tests/motion/tracking/`, built on
  `crate::support::goto::Rig`; `push_tool` in `crates/finder/tests/mcp/`, built on
  `crate::support::mcp`. All under `#[tokio::test(start_paused = true)]` but the tool
  list.
- Vitest for the page, in a new file `TrackPush.test.tsx`. No new Playwright test.

**Target Platform**: the `finder` binary in WSL2 on the Surface.

**Constraints**:
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy and rustdoc, and the red-only night
  mode.
- `crates/finder/web/dist` is committed and must be the build of its source (US3).
- No test moves real hardware.

**Scale/Scope**: one tracker, one session, three stories.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver or `crates/math-core`. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml` or `CODEOWNERS` is edited, but for three `mod` lines that hold no test (spec owner approval 8). |
| III. Tests first, independent expectations | Oracles are the simulator's true eyepiece direction, the session view and the model's epochs. The push is the simulator's own `push_by`, which moves the tube and no count. |
| IV. Critical paths | No `unsafe`, no dependency change, nothing under `proto/`. The state file gains one optional field (spec owner approval 4). |
| V. Proven open source first, honest licensing | Unaffected. |

## Project Structure

```text
crates/finder/
├── src/
│   ├── tracking.rs            # US1: follow, set_model, checks_push, the place read each turn
│   ├── session.rs             # US1: the push check, the push view, the sync rule; US2: going back
│   ├── model.rs               # US1: slip_pending, mark_slip
│   └── mcp/tools.rs           # US2: one sentence in set_tracking's description
├── tests/
│   ├── motion/tracking/
│   │   ├── mod.rs             # US1, US2: one mod line each
│   │   ├── push.rs            # US1, new
│   │   └── push_back.rs       # US2, new
│   └── mcp/
│       ├── main.rs            # US2: one mod line
│       └── push_tool.rs       # US2, new
├── web/
│   ├── src/api.ts             # US3: SessionTrack.push
│   ├── src/components/TrackControl.tsx       # US3
│   ├── src/components/TrackPush.test.tsx     # US3, new
│   └── dist/                  # US3: rebuilt
└── README.md                  # US3
GLOSSARY.md                    # US1
DESIGN.md                      # US3
.impeccable/design.json        # US3
```

## Interfaces

### The tracker (`crates/finder/src/tracking.rs`, US1)

```rust
pub struct State {
    // … every field as today …
    /// Whether the session may check live pointings for a push: true while tracking
    /// with no axis's backlash being measured; false otherwise, `off` and the keyhole
    /// included.
    pub checks_push: bool,
}

impl Tracker {
    /// Hold this J2000 place in place of the target from the rate loop's next turn.
    /// The target keeps its name and takes the place's RA and Dec. Clears the
    /// proportional term, the previous pointing's time and the error samples; keeps the
    /// tracking offset. Refused, changing nothing, unless the state is `tracking` and
    /// no backlash is being measured, both read under the control lock it then writes.
    pub fn follow(&self, ra_deg: f64, dec_deg: f64) -> Result<(), String>;

    /// Use this pointing model for every later rate, forecast and keyhole plan.
    pub fn set_model(&self, model: Model);
}
```

- `State::default()` has `checks_push` false. `Tracker::state()` sets it from the
  control state: true when `state` is `tracking` and both `measurement` slots are
  `None`. A take-up does not count: it does not grow the error
  (`crates/finder/src/tracking.rs:697`).
- The rate loop and the guard read the target from the control state at each turn, not
  from the `target` argument `run` is given (`crates/finder/src/tracking.rs:736`,
  `crates/finder/src/tracking.rs:721`). `Shared::run` and `Shared::guard` lose their
  `target` parameter and read `control.state.target` at each rate-loop turn and guard
  tick; `keyhole` takes the turn's target. `Control` gains `followed: bool`; `follow`
  sets it, and the rate loop's next turn clears it and sets `refresh_at` to 0, so the
  reversal forecasts and the keyhole entry are made again for the new place.
- `Shared.model` becomes `Mutex<Arc<Model>>`, read through a helper that clones the
  `Arc`. `Model::motion` is an `Arc` (`crates/finder/src/model.rs:127`), so a model handed
  over by `set_model` shares the same backlash and crossing state. The keyhole's
  `Planner` borrows the `Arc` it read when the keyhole began.

### The pointing model (`crates/finder/src/model.rs`, US1)

```rust
pub struct Model {
    // … every field as today …
    /// The next sync opens a new zero-point epoch: the axes moved without their
    /// readings following, as after a followed push.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub slip_pending: bool,
}
impl Model {
    pub fn mark_slip(&mut self);
}
```

`add_sync` treats `slip_pending` as a slip (`crates/finder/src/model.rs:497`) and clears
it once the sync is accepted. A refused sync leaves it set. `reset_night` and
`reset_fixed` clear it.

### The session (`crates/finder/src/session.rs`, US1; US2)

```rust
/// Push check bounds (spec owner decisions 1 and 2), in degrees.
const PUSH_THRESHOLD_DEG: f64 = 5.0 / 60.0;
const PUSH_STILL_DEG: f64 = 1.0 / 60.0;
/// Seconds of stillness that settle a push, and the window after a follow.
const PUSH_SETTLE_S: f64 = 5.0;
const PUSH_REOPEN_S: f64 = 10.0;

#[derive(Debug, Clone, Copy, Default)]
enum Push {
    #[default]
    None,
    /// The first pointing of the current still stretch, and its console time.
    Moving {
        ra_deg: f64,
        dec_deg: f64,
        since: f64,
    },
    Following {
        ra_deg: f64,
        dec_deg: f64,
        at: f64,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PushView {
    Moving,
    Following { ra_deg: f64, dec_deg: f64, at: f64 },
}

pub struct TrackView {
    // … every field as today …
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push: Option<PushView>,
}
```

- `TrackRun` gains `push: Push`, `check_armed: bool` and `model_due: bool`, all clear
  in a new `TrackRun`. `stop_tracking` (`crates/finder/src/session.rs:976`), which edits
  the run in place, clears all three too. `set_aim` (`crates/finder/src/session.rs:1271`)
  clears `check_armed`.
- **The effective push** is `run.push` while the run's tracker is not `off`, and
  `Push::None` otherwise. The view, the sync rule and `start_tracking` read only the
  effective push, so a tracker that ended by itself (too low, a lost link, the Sun, a
  power cycle) leaves no push behind (spec FR-009).
- `track_live_pointing` (`crates/finder/src/session.rs:998`) keeps its signature; `now`
  is the latest pointing's `time`, its mid-exposure on the console clock
  (`crates/finder/src/live.rs:202`), and `track_generation` is read in the same lock as
  the tracker:
  1. As today, return unless a tracker is running with a target.
  2. If its state is not `tracking`, clear `check_armed`; if `checks_push` is false or
     `check_armed` is clear, hand the error over as today and set a `Moving` push to
     `None`, setting `check_armed` when the pointing is within `PUSH_THRESHOLD_DEG` of
     the tracker's target and the state is `tracking`; return.
  3. Compute `separation` (`crate::model::separation`, as
     `crates/finder/src/session.rs:871` uses it) of the live pointing from the tracker's
     target.
  4. `Following` within `PUSH_REOPEN_S` of its `at`, and the pointing more than
     `PUSH_STILL_DEG` from the place followed: set `Moving { since: now }` at this
     pointing and mark the model's slip as in step 6; hand nothing over.
  5. Otherwise at or under `PUSH_THRESHOLD_DEG`: a `Moving` push becomes `None`, a
     `Following` push stays; hand the error over as today.
  6. Over it, from `None` or `Following`: set `Moving { since: now }` at this pointing and
     mark the model's slip (persisted as `reset_model` persists, its error logged with
     `eprintln!` and the mark kept in memory). Hand nothing over.
  7. Over it, `Moving`: more than `PUSH_STILL_DEG` from the stretch's first pointing
     starts a new stretch here. Within it, once `now - since >= PUSH_SETTLE_S`, call
     `tracker.follow` with this pointing; if it answers `Ok`, set
     `Following { at: now }` and `model_due`. Hand nothing over.

  Every write of `run.push` re-reads `track_generation` under the values lock and writes
  nothing if it has changed, so a Track run replaced meanwhile is never given another
  run's push. The values lock is never held while calling into the tracker.
- `record_frame_sync` (`crates/finder/src/session.rs:1135`) answers `Ok(false)` before
  anything else while the effective push is `Moving`. After a sync is recorded with
  `model_due` set, it clears `model_due` and, if the tracker is still running, calls
  `tracker.set_model` with the session's model.
- `TrackRun::view` (`crates/finder/src/session.rs:132`) adds `push`, and computes the
  readout's field rotation and longest exposure at the place when `Following`.
- `Session::edit` (`PUT /api/session`) leaves the Track run and its push as they are, as
  015 has it (spec owner decision 10).
- `start_tracking`'s early answer (`crates/finder/src/session.rs:851`) also requires the
  effective push to be `Push::None`, read with `effective_push()` before the block that
  locks the values, since `effective_push()` takes that lock. With a push it goes on as
  for any other start. The old tracker is stopped. Only after stopping an old tracker,
  the session reads both axes through `link.mount()` every 50 ms, for at most 2 s, until
  neither is moving, because StopAll only brakes them and a GoTo refuses a moving mount
  (`crates/finder/src/goto.rs:739`). A missing link or a failed read ends the wait; the
  GoTo then reports it. It answers `Err("stopped")` if `track_generation` has changed
  meanwhile. Then a GoTo starts and Track is armed when the pointing is
  more than 1′ from the target (`crates/finder/src/session.rs:870`). The same wait
  mends 015's Track on for another target while Track is on, which today answers 409
  (spec owner approval 9).

### MCP (`crates/finder/src/mcp/tools.rs`, US2)

`set_tracking`'s description becomes the present sentence followed by a space and
`Turning Track on while it follows a push by hand sends the telescope back to the target.`

### The page (`crates/finder/web/src/`, US3)

```ts
export type SessionTrackPush =
  | { state: "moving" }
  | { state: "following"; ra_deg: number; dec_deg: number; at: number };
// SessionTrack gains: push?: SessionTrackPush;
```

`TrackControl` (`crates/finder/web/src/components/TrackControl.tsx`): while
`track.push` is present, and `session.target` and `track.target` are both present with
the same `id`, a button `Recenter {target}`, placed between the switch's label and the
status paragraph, with the target in italic as the status sets it, calling
`postTrack(true)` with the same pending, refresh and alert handling as the switch. The
button shares the switch's `pending` state, so each disables the other. The status
paragraph and the readout are not changed. The button uses the existing `button` class
without `button-primary` (GoTo's button has both); no new class or token.

In `DESIGN.md`, "push" alone is 022's push-to line in the guidance block (its **Push**
style). The entry for this button calls this spec's push a push by hand.

## Shapes that existing tests pin

- **`Tracker::pointing`'s meaning.** `held_integral_applies_in_deadband_is_clamped_and_clears_on_stop_and_restart`
  (`crates/finder/tests/motion/tracking/pi_loop.rs:411`) feeds errors of 10,000″
  (`crates/finder/tests/motion/tracking/pi_loop.rs:432`) to clamp the offset. The push
  check is in the session, so the tracker still integrates them.
- **The session's `track` view.** `crates/finder/tests/mcp/tracking.rs:245` compares two
  `track` objects whole. `push` is left out when there is none, so both still match.
- **`set_tracking`'s description** must still contain `real telescope`
  (`crates/finder/tests/mcp/tracking.rs:367`).
- **The state file.** `slip_pending` is skipped while false, so a file written today
  and one written after this spec with no pending slip are the same.
- **`TrackControl.test.tsx`** renders sessions with no `push`; their status and the
  switch are unchanged.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| The check in the session | In `Tracker::pointing` | An existing test feeds the tracker large errors on purpose (§ Shapes). The session also owns the sync, which a push has to hold back. |
| Follow in place | End the tracker and start a new one at the place | A restart sends StopAll, loses the learned offset and shows `off` for a moment. |
| Settle on 5 s of stillness | Two pointings in a row within 1′ | Frames come every 500 ms, so a slow nudge settles mid-push and the next 5′ of it are pulled back. |
| Arm the check with a pointing within the threshold | Check every pointing | A keyhole exit or a moved aim point can begin more than 5′ away and would be followed. |
| Mark the slip when moving starts | Mark it on a follow | Going back or Track off while moving would leave the push fitted into the old zero points. |
| The effective push, read through the tracker's state | Clear the push in every ending | The tracker's own endings never reach the session's run. |
| Hand the model over after the first new sync | Keep the tracker's model from Track's start | The keyhole's slew is planned in axis counts through the model, and after a push the old zero points are wrong by the push. |
| A pending slip on the model | Rely on the model's slip check | Its distance is 0.5° with a base tilt and 2.5° before one, so most pushes would be fitted into the night's zero points. |
| Track on means go back | A new route | Track on already means "track the target", and 015 already starts a GoTo when it is far away. |

## User Story 1 — The check and following

Tracker first (`checks_push`, `follow`, `set_model`, the place read each turn), then the
model's mark, then the session's check, view and sync rule, then the glossary term, then
`push.rs`.

## User Story 2 — Going back

The one condition and the wait for rest in `start_tracking`, the description sentence,
`push_back.rs` and `push_tool.rs`.

## User Story 3 — The page

`api.ts`, `TrackControl.tsx`, `TrackPush.test.tsx`, `DESIGN.md`, `.impeccable/design.json`,
the README, then `npm run build` and the rebuilt `dist`. The README describes the push
check, the `push` key, the readout at the place followed, going back, and Track on for
another target while Track is on, in the Track paragraphs under `## API` (after "Track
on requires…") and the `POST /api/track` row.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 86ebeb8, its gates as at 49bb210):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` and
    the two `cargo doc` runs with `RUSTDOCFLAGS="-D warnings"`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results.
- **CI in the merge queue** runs every required `ci.yml` job.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met at 49bb210.
2. **`main` is still as read.** Every citation here is at 86ebeb8, re-read by content on
   2026-11-17 (§ Refinement Record, Re-anchor and Flip). If `main` has moved in these files,
   re-read them and re-run the validator.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Met on 2026-11-17: the owner settled every owner
   decision (spec § Owner decisions), and gave the approvals by flipping it the same day.
4. **Probe builds.** Met on 2026-11-17: two rounds of probe builds (§ Refinement
   Record).
5. **No operator-staged files are needed.**
6. **021 and 022 have landed, and this trio is re-read against them.** Both write
   `crates/finder/src/session.rs`, `crates/finder/web/src/api.ts`,
   `crates/finder/README.md`, `GLOSSARY.md`, `DESIGN.md` and `crates/finder/web/dist`,
   and 021 also writes `crates/finder/src/mcp/tools.rs` and
   `crates/finder/tests/mcp/main.rs`. Met on 2026-11-17: 021 landed at 2f6a777 and 022
   at 20ce2fe, and every anchor was re-read on `main` at 2f6a777 (§ Refinement Record,
   Re-anchor). `depends_on_landed` is left as it is: both are on `main`, so the order
   holds without them. The owner left them out at the flip.
7. **026-mount-sky lands first.** 026 writes `crates/finder/src/session.rs`,
   `crates/finder/web/src/api.ts`, `crates/finder/README.md`, `GLOSSARY.md`,
   `DESIGN.md` and `crates/finder/web/dist`, as US1, US2 and US3 do, and also
   `crates/finder/web/src/useSession.ts` and `crates/finder/web/src/App.tsx`, which
   bring `track` to `TrackControl`. Met at the flip on 2026-11-17: `depends_on_landed`
   gains `026-mount-sky`, so 038 is dispatched only after 026 has landed. 026's US1
   (b9ccf47) and US2 (86ebeb8) are on `main`, and every anchor was re-read there; the
   two `session.rs` citations below 026's `mount_sky_terms` moved 11 lines. 026's US3,
   still to land, writes only the page files above and its banner, and no line of them is
   cited here (§ Refinement Record, Flip).

Outside the factory, before it is relied on at the telescope: GoTo and Track proven on
the real mount, then one push of about 2° and one of about 5′ while tracking, to see that
the first is followed and the second corrected. It needs real stars: a sky drawn from the
axis counts, as 026 draws one, cannot show a push, which moves no count.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** *Check:* every Then names a committed test.
2. **One evidence budget.** *Check:* a story's diff stays under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/038-track-follows-a-push/`.
4. **An earlier story's test file is an existing test.** *Check:* US2 writes
   `push_back.rs` and never `push.rs`.
5. **A new test file at the top of `crates/finder/tests` or in `support/` fails the
   `layout` test.** *Check:* the three new Rust modules sit in their binaries' folders.
6. **An unused helper fails clippy.** *Check:* each module holds only the helpers its own
   tests use; US2 copies what it needs from US1 rather than sharing a file.
7. **The push is in the tube, not the counts.** `SimMount::push_by` changes no axis
   count, so a test that reads counts sees nothing. *Check:* oracles read
   `true_direction`, never axis positions.
8. **Turn is azimuth, not sky, and up is not a test.** A turn of 1° at 43° altitude is
   about 0.73° on the sky. A rate against an axis's tracking direction is sent as zero
   (`crates/finder/src/tracking.rs:873`), so a push up on rising M42 is not pulled back
   quickly even today, and a scenario built on it proves nothing. *Check:* scenarios
   push in tilt, and down.
9. **The live loop hands the pointing over before the sync.** `track_live_pointing`
   runs before `record_frame_sync` for the same frame
   (`crates/finder/src/live.rs:219`, `crates/finder/src/live.rs:226`). *Check:* the push
   state the sync rule reads is the one set for that frame.
10. **Locks.** `track_live_pointing` reads the tracker's state and the session's values
    under two locks. *Check:* in `track_live_pointing`, `record_frame_sync` and
    `start_tracking`, never hold the values lock while calling into the tracker, except
    the early answer's `tracker.state()` read in `start_tracking`, which, like
    `TrackRun::view`'s `tracker.state()` read under it as today, is safe because the
    tracker never locks the session.
11. **The bundle must be rebuilt.** *Check:* US3 commits `crates/finder/web/dist` after
    `npm run build`.
12. **A negative assertion passes when nothing happens.** *Check:* US1 scenario 2's
    "no push" comes with scenario 1's push in the same module, on the same rig, and
    US1-S2, US1-S8 and US2-S3 are written to pass before the change and after it: they
    guard today's behaviour and are not "failing first".
13. **A push state can outlive Track.** `stop_tracking` edits the run in place, and a
    tracker that ends by itself does not touch the run at all
    (`crates/finder/src/tracking.rs:892`). *Check:* every reader goes through the
    effective push, and US1-S4 ends Track while moving.
14. **A sync's time is mid-exposure** (`crates/finder/src/live.rs:202`), earlier than the
    console time at which its frame was handed over. *Check:* US1-S5 compares sync
    times with the push's time, never with the follow's `at`, and US1-S5 and US1-S3
    push just after a new `pointing.seq`, so no frame open at the push is timed after
    it.
15. **Model thinning replaces syncs** (`crates/finder/src/model.rs:512`): a later sync of
    the same epoch within 120 s and 2° replaces an earlier one, so a count of syncs says
    nothing about which were added, and a single read at the follow cannot see a sync
    recorded while moving. *Check:* US1-S5 reads each sync's time and epoch, and US1-S3
    reads the model at every view from the first push until the follow.
16. **A stopped mount still moves.** StopAll brakes; a GoTo refuses a moving axis
    (`crates/finder/src/goto.rs:739`). *Check:* US2's Track on waits for both axes at
    rest, and US2-S1, US2-S2 and US2-S5 fail without the wait; US2-S6 fails without the
    generation recheck.

## Risks

| Risk | Mitigation |
|---|---|
| A real push smears frames for several seconds, so the moving state lasts longer. | It costs nothing: nothing pulls while moving. The field check measures it. |
| A knock of more than 5′ that stays put for 5 s is followed instead of corrected. | One press goes back. The threshold and the settle are constants (owner decisions 1 and 2). |
| 036 lets imaging subs steer the tracker. Its sub errors would pull toward the old place at up to 30″ a pulse after a follow. | 036 is re-anchored to `main` after this lands and must end the subs' claim on a follow. 037's run ends `pushed_away` first in any case. |
| `026-mount-sky` writes `session.rs`, `README.md`, `GLOSSARY.md`, `DESIGN.md`, `api.ts` and `dist`, as US1, US2 and US3 do. | 038 lands after it: 026 is in `depends_on_landed` (§ Prerequisites 7). Its US1 and US2 are on `main` and were re-read; its US3 writes no line cited here. |

## Refinement Record

Drafted on 2026-11-16 by a plate-solver spec session against `main` at 49bb210, from a
reading of `tracking.rs`, `session.rs`, `live.rs`, `model.rs`, the tracking tests and the
simulator's push. Writes come from the task text. Shared writes name the merge edge that
orders them, and the stories that last wrote the same file on `main`. Nothing in this
record is an owner approval: those are in spec.md, and flipping to `ready` gives them.

**Round 1 (probe builds), 2026-11-17.** Each story was built from the draft trio in a
throwaway worktree of `main` at 49bb210, tests first, with no existing test edited and
no undeclared write:

- US1 (`probe/038-r1-us1`, `d0b335f`): 8 tests, every finder test green (374), diff
  40,628 B;
- US2 (`probe/038-r1-us2`, `c59c0f3`, on US1): 4 tests, 378 green, diff 10,828 B; it
  needed the wait for rest in § The session, last bullet, without which Track on
  answers 409;
- US3 (`probe/038-r1-us3`, `f677cf1`, from the base, since US1 and US2 write no web
  file): 9 Vitest tests, the web gate and e2e (29) green, diff 16,995 B.

The fixes are written into this text: 30 s of tracking before any push in "On M42";
`now` is the pointing's own time; step 2 clears a `Moving` push and step 4 marks the
slip; `run` and `guard` lose their `target` parameter; Track on waits for both axes at
rest (owner approval 9, US2-S5); a new target leaves Track and its push, and the button
shows only while the session's target is Track's (owner decision 10, US3-S5); US1-S5
pushes just after a new `pointing.seq`; US1-S3, US1-S7, US2-S1, US2-S3, US3-S3 and
US3-S4 now say what a test can check; US1-S8 passes before the change; the README's
Track text, the glossary's place and two anchors are corrected. No test caught four
behaviours: FR-004's rates at the place followed, FR-007's model hand-over, step 4's
reopen and owner decision 7's kept offset. FR-006 and FR-010 now have assertions
(US1-S3, US1-S1).

**Round 2 (probe builds), 2026-11-17.** Fresh builders worked from the trio as fixed by
round 1 (`70a1a4e`), on `main` at cc593c7, and read nothing of round 1:

- US1 (`probe/038-r2-us1`, `a53731e`): 8 tests, each failing or passing first as T001
  and T002 say; finder tests 374; diff 43,494 B; FR-010 now caught; FR-006 caught only
  once the model is read at each view, as US1-S3 now does;
- US2 (`probe/038-r2-us2`, `7393b3b`, on US1): 5 tests; US2-S5 failed first with
  today's 409; 379 green; diff 11,465 B; no test caught a missing generation recheck,
  which US2-S6 now guards;
- US3 (`probe/038-r2-us3`, `b83b9c8`): 6 Vitest tests; web gate and e2e (29) green;
  diff 18,453 B; US3-S5's condition caught.

No round-2 finding blocks. Its fixes are written into this text: US1-S3 pushes just after
a new `pointing.seq` and reads the model at every view, with a bound that leaves out the
follow's own sync; US2-S6 is new; US2-S3, US2-S4, US2-S5, US3-S3, US3-S4 and US3-S5 now
say what a test can check; the wait for rest runs only after stopping an old tracker,
and a missing link or a failed read ends it; trap 10 names the early answer's read;
FR-012's condition needs both targets present; the button shares the switch's
`pending`; FR-016 names all the README describes; T013 names the fixture and the
component it renders; the `pi_loop.rs` anchor and the `enum Push` block are corrected.
Still caught by no test: the rates and forecasts at the place followed, FR-007's
hand-over, step 4's reopen and owner decision 7's kept offset.

**Re-anchor, 2026-11-17.** 021 landed at 2f6a777 and 022 at 20ce2fe. The trio was
re-read on `main` at 2f6a777, by content. Seven cited lines of `session.rs` (ten
citations) moved 26 lines, and three of `live.rs` (six citations) moved 3; every other
citation reads as before. Neither spec changed a function this one changes, the rig, the
tracking tests, `TrackControl`, its fixture, `SessionTrack`, `set_tracking`'s
description or the gates. Round 2's branches merge onto 2f6a777 with git's own merge but
for `mod push_tool;` beside 021's `mod session_rotation;` and the rebuilt `dist`. The
shared writes below now name 021 and 022 where they wrote a file later; they stay
outside `depends_on_landed` (§ Prerequisites 6). The page's one note on the word "push"
is in § The page; 026's overlap is § Prerequisites 7.

**Flip, 2026-11-17.** The owner flipped 038 with `026-mount-sky` in `depends_on_landed`.
The trio was re-read first on `main` at 86ebeb8, by content, with 026's US1 (b9ccf47) and
US2 (86ebeb8) landed. Two cited lines of `session.rs` moved 11 lines, below US1's
`mount_sky_terms`; every other citation reads as before. The shared writes below now name
026 where it writes a file last: US1 for `session.rs`, US2 for `GLOSSARY.md` and the
README, and US3, still to land, for `DESIGN.md`, `api.ts` and `dist`.

```yaml
version: 1
reviewed_revision: 86ebeb87f0152d4656bc0e2a2b4d44a0a6df7817
stories:
  US1:
    tests:
      - path: crates/finder/tests/motion/tracking/push.rs
        status: new
        disposition: add
        assertion: "a 2° push down is followed at least 5 s after it: push.state following, its place within 60″ of the true place, the readout's field rotation computed at the place followed within 1e-9 (FR-010), and for 120 s the eyepiece stays within 60″ of that place and more than 1.9° from M42"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/motion/tracking/push.rs
        status: new
        disposition: add
        assertion: "a 3′ push is corrected within 30 s with no push key; a push in two steps 1 s apart, the first just after a new pointing.seq, shows moving from the first view with a push until the follow, never pulls back by more than 1′, has at every view from the first step until the follow no sync timed after the first step and before push.at (FR-006), and follows the final place at least 5 s after the second step"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/motion/tracking/push.rs
        status: new
        disposition: add
        assertion: "Track off while moving clears the push and lets a sync in within 10 s; a 15′ push, under this rig's 2.5° slip distance, adds exactly one epoch with every earlier epoch's syncs before the push and the newest epoch's after it; a follow below the minimum altitude ends too_low with no push key and both axes at rest; checks_push is false during a backlash measurement and true after; an aim change of about 20′ shows no push for 60 s"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/motion/tracking/pi_loop.rs
        status: existing
        disposition: preserve
        assertion: "the tracker still integrates and clamps errors of 10,000″ handed to pointing directly"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Tracker::pointing is aim point minus target, corrected as 015 has it (015 FR-008)"
      - "a session read with no push has the same track object as before (spec.md SC-004)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion tracking::push::"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/src/model.rs
      - crates/finder/src/session.rs
      - crates/finder/src/tracking.rs
      - crates/finder/tests/motion/tracking/mod.rs
      - crates/finder/tests/motion/tracking/push.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/session.rs
        other_spec: 038-track-follows-a-push
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and changes start_tracking"
      - path: crates/finder/tests/motion/tracking/mod.rs
        other_spec: 038-track-follows-a-push
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds mod push_back"
      - path: crates/finder/src/session.rs
        other_spec: 026-mount-sky
        other_story: US1
        disposition: merge_order
        reason: "038 depends_on_landed 026, whose US1 last changed session.rs (b9ccf47); 028's US4, 022's US3 and 021's US1 wrote it earlier, and are on main"
      - path: crates/finder/src/tracking.rs
        other_spec: 024-mount-link-recovery
        other_story: US3
        disposition: merge_order
        reason: "038 depends_on_landed 024, whose US3 last changed tracking.rs"
      - path: crates/finder/src/model.rs
        other_spec: 015-tracking
        other_story: US5
        disposition: merge_order
        reason: "038 depends_on_landed 015, whose US5 last changed model.rs"
      - path: GLOSSARY.md
        other_spec: 026-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "038 depends_on_landed 026, whose US2 last changed GLOSSARY.md (86ebeb8); 025's US2, 022's US1 and 021's US1 wrote it earlier, and are on main"
      - path: crates/finder/tests/motion/tracking/mod.rs
        other_spec: 027-finder-tests-by-system
        other_story: US2
        disposition: merge_order
        reason: "038 depends_on_landed 027, whose US2 last changed the module list"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/motion/tracking/push_back.rs
        status: new
        disposition: add
        assertion: "Track on while following, or while moving by hand, answers armed with no push key, starts a GoTo and ends tracking with no push key and the eyepiece within 60″ of M42; Track on with no push answers with no goto key and keeps the same tracker; Track on after a new target of M43 answers armed for M43 with a GoTo under way and ends tracking within 60″ of it; Track off 20 ms into the wait for rest makes Track on answer 409 stopped, with no GoTo in the next 5 s and both axes at rest"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/mcp/push_tool.rs
        status: new
        disposition: add
        assertion: "set_tracking's description contains real telescope and is exactly the description before this spec, a space, and the sentence of FR-011"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/mcp/tracking.rs
        status: existing
        disposition: preserve
        assertion: "set_tracking's description contains real telescope, and two track objects with no push compare equal"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Track on for the target already tracked with no push answers at once and changes nothing, with the same tracker (crates/finder/src/session.rs:851)"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test motion tracking::push_back::"
        - "cargo nextest run -p finder --locked --test mcp push_tool::"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/mcp/tools.rs
      - crates/finder/src/session.rs
      - crates/finder/tests/mcp/main.rs
      - crates/finder/tests/mcp/push_tool.rs
      - crates/finder/tests/motion/tracking/mod.rs
      - crates/finder/tests/motion/tracking/push_back.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "038 depends_on_landed 017, whose US1 changed tools.rs; 021's US1 wrote it later, and is on main"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 027-finder-tests-by-system
        other_story: US2
        disposition: merge_order
        reason: "038 depends_on_landed 027, whose US2 changed the module list; 021's US1 added a line later, and is on main"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/web/src/components/TrackPush.test.tsx
        status: new
        disposition: add
        assertion: "a Recenter M42 button while a push is moving or followed, with the status unchanged, that posts Track on once and keeps the switch on, refreshing the session once after a 200 and showing a 409's error as an alert; without a push, no Recenter button, the switch the only button element and no element with the role button; and no Recenter button while the session's target is not Track's"
        decision: "spec.md:US3-S1"
      - path: crates/finder/web/src/components/TrackControl.test.tsx
        status: existing
        disposition: preserve
        assertion: "the switch, status and readout of sessions with no push are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the Track switch and its readout read as before when there is no push (DESIGN.md, Track and tracking readout)"
    checks:
      development:
        - "npm test -- TrackPush"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - .impeccable/design.json
      - DESIGN.md
      - crates/finder/README.md
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/TrackControl.tsx
      - crates/finder/web/src/components/TrackPush.test.tsx
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 026-mount-sky
        other_story: US2
        disposition: merge_order
        reason: "038 depends_on_landed 026, whose US2 last changed the README (86ebeb8); 025's US2, 022's US3 and 021's US1 wrote it earlier, and are on main"
      - path: DESIGN.md
        other_spec: 026-mount-sky
        other_story: US3
        disposition: merge_order
        reason: "038 depends_on_landed 026, whose US3 changes DESIGN.md; 025's US3, 022 and 021 wrote it earlier, and are on main"
      - path: .impeccable/design.json
        other_spec: 025-frame-capture
        other_story: US3
        disposition: merge_order
        reason: "038 depends_on_landed 025, whose US3 last changed design.json"
      - path: crates/finder/web/dist
        other_spec: 026-mount-sky
        other_story: US3
        disposition: merge_order
        reason: "038 depends_on_landed 026, whose US3 rebuilds dist; 025's US3, 022 and 021 rebuilt it earlier, and are on main"
      - path: crates/finder/web/src/api.ts
        other_spec: 026-mount-sky
        other_story: US3
        disposition: merge_order
        reason: "038 depends_on_landed 026, whose US3 changes api.ts; 025's US3, 022's US1 and US3 and 021's US1 wrote it earlier, and are on main"
      - path: crates/finder/web/src/components/TrackControl.tsx
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "038 depends_on_landed 015, whose US8 last changed TrackControl.tsx"
    open_questions: []
```
