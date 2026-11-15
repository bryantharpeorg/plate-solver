---
description: "Task list for 025-frame-capture: Saving Live Frames at the Telescope"
---

# Tasks: Saving Live Frames at the Telescope

**Prerequisites** (plan.md § Prerequisites):
- `013-live-finder`, `014-goto`, `016-mcp`, `017-console-stop-confirm`,
  `019-night-legibility` and `020-live-chart-calm` have landed on `main`;
- the owner decisions in spec.md are settled and its owner approvals given;
- `024-mount-link-recovery` has landed and this trio has been re-anchored to it
  (plan.md § Prerequisites 6);
- `027-finder-tests-by-system` has landed and this trio has been refreshed against its
  test layout: met, at b8dceaa (plan.md § Prerequisites 7);
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
  The one exception is the `mod` line of each new finder test module, added in
  alphabetical order to the binary's `main.rs` your story's tasks name, which holds no
  test (027's owner approval 7). Change nothing else there.
- New tests go in new files: the ones your story's tasks name. A new finder test is a
  module file in the binary's folder plan.md § Technical Context names, never a file at
  the top level of `crates/finder/tests` or in `crates/finder/tests/support`, either of
  which fails the `layout` test. It binds the live helpers with
  `use crate::support::live as support;`, never with `#[path]`, and holds only the
  setup its own tests use, since an unused helper fails `clippy -D warnings`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Tests compute their expected values themselves (plan.md § Plan traps 6), and compare
  numbers read back from a file after the `serde_json` round trip of spec.md § User
  Scenarios, exactly. The only tolerances are US1-S3's 0.01° and 0.05 s, which bound a
  fresh solve of a replayed frame.
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 3 says.
- Every code citation is `path:line` as read at c1ed295 and re-checked at f8dab49 and
  1104b7d, and those under `crates/finder/tests` re-read at b8dceaa, after 027 moved
  them. If a cited line has moved, re-read it rather than guess.
- Add no dependency and change no `Cargo.toml`. Change nothing under
  `crates/simulator`, and outside US2 nothing under `crates/scope-node` or
  `crates/scope-link`.
- Do not edit `specs/025-frame-capture/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — Record a stretch of the night on the console (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/console/recording.rs`, the module
  `recording` of finder's `console` test binary, and add `mod recording;` after
  `mod rates;` in `crates/finder/tests/console/main.rs`. Give it the shared setup of
  spec.md § User Scenarios: the `Simulator` of `crates/finder/tests/support/live.rs`
  with a `SimMount` connected first and `push_to(50.0, 120.0)` after, a `SimNode`
  served with `memory::serve_routes_on` and `routes()` as
  `crates/finder/tests/support/mcp.rs` serves its node, the paired session,
  `with_live()` and `with_recordings(R)` with R under `target/` and not made by the
  test, under `#[tokio::test(start_paused = true)]`, and helpers to wait on the session,
  to read the folder's lines with `serde_json` and to pass a held value through the
  `serde_json` round trip. Write failing tests for US1-S1 and US1-S2.
- [ ] T002 [US1] Add failing tests to `crates/finder/tests/console/recording.rs` for the
  replay of a recorded frame through a new `Live` (US1-S3) and for the refusals (US1-S4).
- [ ] T003 [US1] Add failing tests to `crates/finder/tests/console/recording.rs` for a
  repeated start (US1-S5), the frame limit and a deleted folder (US1-S6), and a frame id
  that does not rise, fed to a `Recorder` the test starts with channels of its own
  (US1-S8). That test builds `Sources` by literal and reads `RecordingView` field by
  field, never building or comparing a whole one, since US2 adds `raw` (plan.md
  § Shapes that existing tests pin). Create `crates/finder/tests/cli/recording.rs`, the
  module `recording` of `cli`, with `mod recording;` after `mod flags;` in
  `crates/finder/tests/cli/main.rs`, and in it a failing test for the built binary under
  `--simulate --state` (US1-S7).

### Implementation

- [ ] T004 [US1] Create `crates/finder/src/recording.rs` with `Recorder`, the
  `#[non_exhaustive]` `RecordingView`, `Sources`, `FolderError`, the folder name and the
  recording task,
  exactly as plan.md § Interfaces gives them, with every write, count and end under the
  run's one lock, `stop` ending before it aborts the task, and the `node_restarted`
  check under that lock before a frame's preview or line is written, and export it from
  `crates/finder/src/lib.rs` with the route's doc line.
- [ ] T005 [US1] In `crates/finder/src/api.rs`, add `with_recordings`,
  `with_recording_limit`, the `/api/record` route and the `recording` field of
  `SessionResponse`. In `crates/finder/src/main.rs`, pass `recordings/` in the state
  folder to `with_recordings`.
- [ ] T006 [US1] Add the route's row and the `## Recording a night` section to
  `crates/finder/README.md`, and the **Recording** entry to `GLOSSARY.md`
  (plan.md § Interfaces).
- [ ] T007 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6, US1-S7
  and US1-S8).

## Phase 2: User Story 2 — Save the scope node's raw frames into the recording (Priority: P2)

### Tests first

- [ ] T008 [US2] Create `crates/scope-node/tests/raw_frame.rs` with a `NodeService` over
  a `SyntheticCamera` on `SystemClock` served with `routes()` over
  `scope_link::memory::channel_pair`, as `crates/simulator/tests/stop_finishes.rs`
  serves one. Write failing tests for US2-S1 and US2-S2.
- [ ] T009 [US2] Create `crates/finder/tests/console/recording_raw.rs`, the module
  `recording_raw` of `console`, with `mod recording_raw;` right after US1's
  `mod recording;` in `crates/finder/tests/console/main.rs`, and the shared setup of
  spec.md § User Scenarios. Write failing tests for the saved raw frames (US2-S3); the
  refusals, a console without a recorder, a node sending no frame and a stop during a
  request (US2-S4); a node without the service and a reply of 17 bytes for a 4 × 4
  frame (US2-S5); and a 2592 × 1944 frame from a camera of the test's own (US2-S6).
  Create `crates/finder/tests/motion/recording_raw.rs`, the module `recording_raw` of
  `motion`, with `mod recording_raw;` after `mod night;` in
  `crates/finder/tests/motion/main.rs`, holding the part of the shared setup it uses
  and a failing test for raw frames refused while the mount moves and stopped with
  `the mount started moving` when a move starts (US2-S7).

### Implementation

- [ ] T010 [US2] Add `ScopeNodeRaw`, `RawFrameRequest` and `RawFrameReply` to
  `crates/scope-link/proto/scope_node/v1/scope_node.proto`, changing nothing already
  there. In `crates/scope-node/src/service.rs`, add the raw waiters to `Commands`,
  answer them in `run`, implement `ScopeNodeRaw` and serve it from `routes`
  (plan.md § Interfaces).
- [ ] T011 [US2] In `crates/finder/src/link.rs`, add `RAW_MESSAGE_LIMIT`, `RawClient`,
  `LinkManager::raw` and the private `Paired` struct, publishing the raw client where
  the mount client is published.
- [ ] T012 [US2] In `crates/finder/src/recording.rs`, add the `#[non_exhaustive]`
  `RawView`, `RawRefusal`, `Moving`, `request_raw` and the raw task, checking each
  reply's length before `GrayImage::from_raw` and setting each `error`, like `saved`,
  under the run's lock and only while the run is on. Add no field to `Sources` and
  change no signature US1's test calls. In `crates/finder/src/api.rs`, move the motion test into
  `motion_running` and add the `/api/record/raw` route with its `moving` closure, which
  calls `motion_running` (plan.md § The raw route).
- [ ] T013 [US2] Add the raw route's row and paragraph to `crates/finder/README.md`, its
  line to the route list in `crates/finder/src/lib.rs`, and the **Raw frame** entry to
  `GLOSSARY.md` (plan.md § Interfaces).
- [ ] T014 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6 and
  US2-S7).

## Phase 3: User Story 3 — Start and stop a recording from the Setup page (Priority: P3)

### Tests first

- [ ] T015 [US3] Create `crates/finder/web/src/components/RecordingSection.test.tsx` with
  failing tests for US3-S1, US3-S2, US3-S3, US3-S4, US3-S5 and US3-S6, stubbing `fetch`
  and `EventSource` as the existing Setup page tests do.

### Implementation

- [ ] T016 [US3] Add `SessionRecording`, `postRecord` and `postRecordRaw` to
  `crates/finder/web/src/api.ts`, copy `recording` in `request()` of
  `crates/finder/web/src/useSession.ts`, create
  `crates/finder/web/src/components/RecordingSection.tsx` with the words of plan.md
  § Interfaces, and place it in `crates/finder/web/src/components/SetupPage.tsx` after
  the aim tap.
- [ ] T017 [US3] Add the Recording entry to `DESIGN.md` and the Recording component to
  `.impeccable/design.json`. Run the `lint` and `test` gates with every existing Vitest
  and Playwright test unchanged and green, commit the rebuilt `crates/finder/web/dist`,
  and commit the green story (US3-S1, US3-S2, US3-S3, US3-S4, US3-S5 and US3-S6).

## Execution Order

```text
US1 ──► US2 ──► US3
```

- **US1** comes first: US2 writes into its recordings and extends its `Recorder`, route
  table, route list, README section and glossary, and adds its `mod` line right after
  US1's in `crates/finder/tests/console/main.rs`.
- **US2** comes next: it adds the raw route and `recording.raw`, which US3's page calls
  and shows.
- **US3** comes last and touches only `crates/finder/web/`, `DESIGN.md` and
  `.impeccable/design.json`.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
