---
description: "Task list for 025-frame-capture: Saving Live Frames at the Telescope"
---

# Tasks: Saving Live Frames at the Telescope

**Prerequisites** (plan.md § Prerequisites):
- `013-live-finder`, `014-goto`, `016-mcp`, `017-console-stop-confirm` and
  `019-night-legibility` have landed on `main`;
- the owner decisions in spec.md are settled and its owner approvals given;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
- New tests go in new files: the ones your story's tasks name.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Tests compute their expected values themselves (plan.md § Plan traps 6).
- Keep the story's diff under 64 KiB, measured as plan.md § Plan traps 3 says.
- Every code citation is `path:line` as read at c1ed295. If a cited line has moved,
  re-read it rather than guess.
- Add no dependency and change no `Cargo.toml`. Change nothing under
  `crates/simulator`, and outside US2 nothing under `crates/scope-node` or
  `crates/scope-link`.
- Do not edit `specs/025-frame-capture/`.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — Record a stretch of the night on the console (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/recording.rs` with the shared setup of
  spec.md § User Scenarios: the `Simulator` of `crates/finder/tests/live/support.rs`
  with a `SimMount` connected, a `SimNode` served with `routes()` as
  `crates/finder/tests/mcp/support.rs` serves its node, the paired session, `with_live()` and
  `with_recordings(R)` with R under `target/`, and helpers to wait on the session and
  to read the folder's lines with `serde_json`. Write failing tests for US1-S1 and
  US1-S2.
- [ ] T002 [US1] Add failing tests to `crates/finder/tests/recording.rs` for the replay
  of a recorded frame through a new `Live` (US1-S3) and for the refusals (US1-S4).
- [ ] T003 [US1] Add failing tests to `crates/finder/tests/recording.rs` for a repeated
  start (US1-S5), the frame limit and a deleted folder (US1-S6), and the built binary
  under `--simulate --state` (US1-S7).

### Implementation

- [ ] T004 [US1] Create `crates/finder/src/recording.rs` with `Recorder`,
  `RecordingView`, `Sources`, `FolderError`, the folder name and the recording task,
  exactly as plan.md § Interfaces gives them, and export it from
  `crates/finder/src/lib.rs` with the route's doc line.
- [ ] T005 [US1] In `crates/finder/src/api.rs`, add `with_recordings`,
  `with_recording_limit`, the `/api/record` route and the `recording` field of
  `SessionResponse`. In `crates/finder/src/main.rs`, pass `recordings/` in the state
  folder to `with_recordings`.
- [ ] T006 [US1] Add the route's row and the `## Recording a night` section to
  `crates/finder/README.md`, and the **Recording** entry to `GLOSSARY.md`
  (plan.md § Interfaces).
- [ ] T007 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6 and
  US1-S7).

## Phase 2: User Story 2 — Save the scope node's raw frames into the recording (Priority: P2)

### Tests first

- [ ] T008 [US2] Create `crates/scope-node/tests/raw_frame.rs` with a `NodeService` over
  a `SyntheticCamera` served with `routes()` over `scope_link::memory::channel_pair`, as
  `crates/simulator/tests/stop_finishes.rs` serves one. Write failing tests for US2-S1
  and US2-S2.
- [ ] T009 [US2] Create `crates/finder/tests/recording_raw.rs` with the shared setup of
  spec.md § User Scenarios. Write failing tests for the saved raw frames (US2-S3), the
  refusals and a stop during a request (US2-S4), a node without the service (US2-S5)
  and a 2592 × 1944 frame from a camera of the test's own (US2-S6).

### Implementation

- [ ] T010 [US2] Add `ScopeNodeRaw`, `RawFrameRequest` and `RawFrameReply` to
  `crates/scope-link/proto/scope_node/v1/scope_node.proto`, changing nothing already
  there. In `crates/scope-node/src/service.rs`, add the raw waiters to `Commands`,
  answer them in `run`, implement `ScopeNodeRaw` and serve it from `routes`
  (plan.md § Interfaces).
- [ ] T011 [US2] In `crates/finder/src/link.rs`, add `RAW_MESSAGE_LIMIT`, `RawClient`,
  `LinkManager::raw` and the private `Paired` struct, publishing the raw client where
  the mount client is published.
- [ ] T012 [US2] In `crates/finder/src/recording.rs`, add `RawView`, `RawRefusal`,
  `request_raw` and the raw task. In `crates/finder/src/api.rs`, add the
  `/api/record/raw` route.
- [ ] T013 [US2] Add the raw route's row and paragraph to `crates/finder/README.md`, and
  the **Raw frame** entry to `GLOSSARY.md` (plan.md § Interfaces).
- [ ] T014 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1, US2-S2, US2-S3, US2-S4, US2-S5 and US2-S6).

## Phase 3: User Story 3 — Start and stop a recording from the Setup page (Priority: P3)

### Tests first

- [ ] T015 [US3] Create `crates/finder/web/src/components/RecordingSection.test.tsx` with
  failing tests for US3-S1, US3-S2, US3-S3 and US3-S4, stubbing `fetch` and
  `EventSource` as the existing Setup page tests do.

### Implementation

- [ ] T016 [US3] Add `SessionRecording`, `postRecord` and `postRecordRaw` to
  `crates/finder/web/src/api.ts`, create
  `crates/finder/web/src/components/RecordingSection.tsx` with the words of plan.md
  § Interfaces, and place it in `crates/finder/web/src/components/SetupPage.tsx` after
  the aim tap.
- [ ] T017 [US3] Add the Recording entry to `DESIGN.md`. Run the `lint` and `test` gates
  with every existing Vitest and Playwright test unchanged and green, commit the rebuilt
  `crates/finder/web/dist`, and commit the green story (US3-S1, US3-S2, US3-S3 and
  US3-S4).

## Execution Order

```text
US1 ──► US2 ──► US3
```

- **US1** comes first: US2 writes into its recordings and extends its `Recorder`, route
  table, README section and glossary.
- **US2** comes next: it adds the raw route and `recording.raw`, which US3's page calls
  and shows.
- **US3** comes last and touches only `crates/finder/web/` and `DESIGN.md`.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
