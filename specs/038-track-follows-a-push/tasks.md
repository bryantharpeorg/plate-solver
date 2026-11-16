---
description: "Task list for 038-track-follows-a-push: Track Follows a Push"
---

# Tasks: Track Follows a Push

**Prerequisites** (plan.md § Prerequisites):
- every spec in `depends_on_landed` has landed on `main`: met at 49bb210;
- the owner decisions in spec.md are settled and its owner approvals given: not met;
- every story has been probe-built: not met;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. The one exception
  is the `mod` line your story's tasks name, added in alphabetical order (spec.md owner
  approval 8). A test file an earlier story of this spec created is an existing test.
- A new finder test is a module file in its binary's folder, never a file at the top of
  `crates/finder/tests` or in `crates/finder/tests/support`, either of which fails the
  `layout` test. It holds only the helpers its own tests use.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- `crates/math-core`, `crates/simulator`, `crates/scope-node`, `crates/scope-link` and
  `proto/` are never touched.
- No test moves real hardware or calls a real LLM.
- Run no tool the gates don't run.

## Phase 1: User Story 1 — A push of more than 5′ is followed, a smaller one is corrected (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/motion/tracking/push.rs` and add `mod push;`
  to `crates/finder/tests/motion/tracking/mod.rs`. Build the rig and "on M42" as
  spec.md § User Scenarios defines them, with a helper for the true place from
  `rig.mount.true_direction()`. Write failing tests for US1-S1, US1-S3, US1-S4, US1-S5,
  US1-S6 and US1-S8, and the test for US1-S2, which passes before the change too and
  guards today's correction of a small push (plan.md § Plan traps 12). Every push is
  down in tilt (plan.md § Plan traps 8).
- [ ] T002 [US1] Add a failing test to `push.rs` for US1-S7, building a tracker on M81 as
  `crates/finder/tests/motion/tracking/backlash.rs:9` does and reading `checks_push`
  every second across the first azimuth reversal.

### Implementation

- [ ] T003 [US1] In `crates/finder/src/tracking.rs`: add `State::checks_push` and
  `Control::followed`; read the target from the control state
  at each turn of the rate loop and the guard; make `Shared.model` a `Mutex<Arc<Model>>`;
  add `Tracker::follow` and `Tracker::set_model` (plan.md § The tracker).
- [ ] T004 [US1] In `crates/finder/src/model.rs`: add `Model::slip_pending`,
  `Model::mark_slip`, and the slip rule in `add_sync`; clear it in `reset_night` and
  `reset_fixed` (plan.md § The pointing model).
- [ ] T005 [US1] In `crates/finder/src/session.rs`: add the push constants, `Push`,
  `PushView`, `TrackRun::push`, `TrackRun::check_armed`, `TrackRun::model_due` and
  `TrackView::push`; clear them in `stop_tracking` and `check_armed` in `set_aim`; the
  effective push; steps 1 to 7 of `track_live_pointing` with the generation recheck;
  the sync rule and the model hand-over in `record_frame_sync`; `push` and the readout
  at the place in `TrackRun::view` (plan.md § The session, every bullet but the last,
  which is US2's).
- [ ] T006 [US1] Add **Followed push** to `GLOSSARY.md` in the words of FR-015.
- [ ] T007 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-001 to FR-007, FR-009, FR-010, FR-015;
  US1-S1 to US1-S8).

## Phase 2: User Story 2 — Going back to the target (Priority: P1)

### Tests first

- [ ] T008 [US2] Create `crates/finder/tests/motion/tracking/push_back.rs` and add
  `mod push_back;` after `mod push;`. Copy the rig helpers it needs from `push.rs`. Write
  failing tests for US2-S1 and US2-S2, and the test for US2-S3, which passes before the
  change too and guards today's no-op.
- [ ] T009 [US2] Create `crates/finder/tests/mcp/push_tool.rs` and add `mod push_tool;`
  to `crates/finder/tests/mcp/main.rs`. Write a failing test for US2-S4.

### Implementation

- [ ] T010 [US2] In `crates/finder/src/session.rs`, make `start_tracking`'s early answer
  also require `Push::None` (plan.md § The session, last bullet).
- [ ] T011 [US2] Append the sentence of FR-011 to `set_tracking`'s description in
  `crates/finder/src/mcp/tools.rs`.
- [ ] T012 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green, then commit the green story (FR-008, FR-011; US2-S1 to US2-S4).

## Phase 3: User Story 3 — The page shows the push and offers the way back (Priority: P2)

### Tests first

- [ ] T013 [US3] Create `crates/finder/web/src/components/TrackPush.test.tsx` in the
  shape of `TrackControl.test.tsx`, with failing tests for US3-S1, US3-S2 and US3-S3,
  and the test for US3-S4, which passes before the change too.

### Implementation

- [ ] T014 [US3] Add `SessionTrackPush` and `SessionTrack.push` to
  `crates/finder/web/src/api.ts`, and the `Recenter {target}` button to
  `crates/finder/web/src/components/TrackControl.tsx` (plan.md § The page).
- [ ] T015 [US3] Record the button in `DESIGN.md`'s "Track and tracking
  readout" section and in the "Track and Tracking Readout" component of
  `.impeccable/design.json`.
- [ ] T016 [US3] Describe the push check, the `push` key and going back in the Track
  section and the `POST /api/track` row of `crates/finder/README.md`.
- [ ] T017 [US3] Run `npm run build` in `crates/finder/web` and commit `dist`. Run the
  `lint` and `test` gates with every existing test unchanged and green, then commit the
  green story (FR-012 to FR-014, FR-016; US3-S1 to US3-S4).

## Execution Order

The stories land in this order:

```text
US1 → US2 → US3
```

- **US2** starts from a `main` that holds US1's push state in `session.rs` and its
  `mod push;` line.
- **US3** starts from a `main` that holds US2, so the README describes going back as
  built.
- **`[P]`** marks nothing here: every task within a story shares a file with another.
