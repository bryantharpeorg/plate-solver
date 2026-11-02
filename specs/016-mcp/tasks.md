---
description: "Task list for 016-mcp: The Console's MCP Server"
---

# Tasks: The Console's MCP Server

**Prerequisites** (plan.md § Prerequisites):
- `012-targets-and-push-to`, `013-live-finder`, `014-goto` and `015-tracking` have
  landed on `main`, and the trio has been re-read against 014's US9 and 015 as landed;
- the owner questions in plan.md § Refinement Record are answered, and the owner
  approvals in spec.md are given, including the `rmcp` and `base64` dependencies;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml` or
  `CODEOWNERS`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates (plan.md § Plan traps,
  trap 18).
- New Rust tests go in new files under `crates/finder/tests/`, or in-module. Shared test
  code goes in `crates/finder/tests/mcp/support.rs`, which starts with
  `#![allow(dead_code)]` as `tests/live/support.rs` does.
- `crates/math-core`, `proto/`, `crates/simulator`, `crates/scope-node`, the scope link
  and its proto are never touched.
- No test moves real hardware or calls a real LLM. The mount is reached only through the
  simulator, on its virtual clock.
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane` (plan.md § Plan traps, trap 5).

## Phase 1: User Story 1 — The MCP endpoint, and the door that guards it (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/mcp/support.rs` with a request helper that
  posts JSON-RPC to `/mcp` through `finder::router` with `tower::ServiceExt::oneshot`, a
  peer set through axum's `MockConnectInfo`, and spec.md's shared headers unless a test
  overrides them. Write failing tests in a new `crates/finder/tests/mcp_endpoint.rs`:
  - `initialize` with no `Authorization`, then `tools/list` with no `Mcp-Session-Id`,
    naming `get_session` (read-only hint) and `set_target` (US1-S1);
  - GET gets 405, with the state file byte-for-byte unchanged (US1-S2);
  - with MCP off, a POST to `/mcp` answers as the same POST to `/no-such-path` (US1-S3);
  - `MCP-Protocol-Version: 1999-01-01` gets 400 (US1-S4);
  - a notification gets 202 (US1-S5).
- [ ] T002 [US1] Add failing door tests to `crates/finder/tests/mcp_endpoint.rs`:
  - a foreign `Origin` gets 403, with the target unchanged (US1-S6);
  - DNS rebinding with `evil.example:8080` gets 403 (US1-S7);
  - `192.168.1.20:8080`, and `surface.local:8080` under `--mcp-allow-host surface.local`,
    are answered 200 (US1-S8);
  - the peers `127.0.0.1`, `192.168.1.30`, `172.24.0.1`, `fe80::1`, `fd00::30` and
    `::ffff:192.168.1.30` are answered 200 (US1-S9);
  - the peers `203.0.113.7` and `::ffff:203.0.113.7` get 403 for `get_session` and
    `set_target` (US1-S10);
  - a router served without connect info gets 403 (US1-S15).
- [ ] T003 [US1] Add failing tests to `crates/finder/tests/mcp_endpoint.rs`, on a console
  with `session::FixedClock`:
  - `get_session` and `set_target` equal their routes' JSON, as `structuredContent` and as
    text (US1-S11);
  - `GET /api/mcp` on three consoles, with an injected address list (US1-S12).

### Implementation

- [ ] T004 [US1] Add `rmcp = { version = "=3.5.1", default-features = false, features =
  ["server", "macros", "transport-streamable-http-server"] }` to
  `crates/finder/Cargo.toml`. Update `Cargo.lock` by running cargo once without
  `--locked`, then `cargo update -p rmcp-macros --precise 3.5.1` (owner question 4).
  Compare the lock's diff with plan.md § Dependencies: the 34 packages and the listed
  changes, and nothing else. Don't edit `deny.toml`; its duplicate-version warnings are
  accepted (US1-S13).
- [ ] T005 [US1] Add `pub mod mcp;` to `crates/finder/src/lib.rs`. Create
  `crates/finder/src/mcp/mod.rs` with `McpConfig` (enabled, allowed host names, the bound
  address and the address source of plan.md § `GET /api/mcp`) and the `rmcp` service,
  built with the builder calls in plan.md § `rmcp` settings and mounted with
  `route_service`.
- [ ] T006 [US1] In `crates/finder/src/api.rs`, move the bodies the US1 tools share into
  crate functions that the routes then call (plan.md § The shared functions):
  `session_response` from `get_session`, `session_edit_error` and `edit_session` from
  `put_session` (which keeps its `Json` extractor), and `check_origin` and
  `host_allowed` from `SessionWrite`. No route's answer changes.
- [ ] T007 [US1] Create `crates/finder/src/mcp/door.rs`: axum middleware that reads the
  body up to 44 MiB and makes the `Origin`, `Host` and peer checks in plan.md's order,
  through `check_origin` and `host_allowed`, judging an IPv4-mapped peer as IPv4 and
  refusing a request with no `ConnectInfo`. A failed check answers 403 with a JSON-RPC
  error and no id; otherwise the request goes on unchanged.
- [ ] T008 [US1] Create `crates/finder/src/mcp/tools.rs` with the `Console` server
  (`#[tool_router]`, `#[tool_handler]`, `ServerConfig` with the `tools` capability) and
  `get_session` and `set_target`. `get_session` calls `session_response`; `set_target`
  reads its argument with `axum::Json::<SessionEdit>::from_bytes` and
  `session_edit_error`, then calls `edit_session`. Each returns its JSON as text and as
  `structuredContent` under FR-004's object rule, with `outputSchema`
  `{"type": "object"}` (owner question 9). Refusals are `isError: true` with the route's
  reason.
- [ ] T009 [US1] In `crates/finder/src/api.rs`:
  - add `AppState::with_mcp(McpConfig)`, with MCP on by default;
  - mount the service at `/mcp` behind the door with `from_fn_with_state` unless MCP is
    off;
  - add `GET /api/mcp` (plan.md § `GET /api/mcp`), with the production address source
    and the trait the tests replace.
- [ ] T010 [US1] In `crates/finder/src/main.rs`, add `--no-mcp` and `--mcp-allow-host`
  (repeatable), build `AppState::with_mcp(..)`, record the bound address after binding,
  and serve with `into_make_service_with_connect_info::<SocketAddr>()`. Add no option that
  widens the listen address (US1-S14).
- [ ] T011 [US1] Run the `lint` and `test` gates (plan.md § Verification). Confirm
  `tests/api.rs`, `tests/cli.rs`, `tests/session_api.rs` and `tests/session_writes.rs`
  are unchanged and green, and that the diff is under 64 KiB (plan.md § Plan traps, trap
  3). Commit the green story (FR-001, FR-002, FR-003, FR-004, FR-005, FR-016, FR-019;
  US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6, US1-S7, US1-S8, US1-S9, US1-S10,
  US1-S11, US1-S12, US1-S13, US1-S14, US1-S15).

## Phase 2: User Story 2 — The session and catalog tools (Priority: P1)

### Tests first

- [ ] T012 [US2] Add to `crates/finder/tests/mcp/support.rs` the simulator rig: the
  simulated mount wrapped in a recording mount that logs each go-to, rate and stop all
  with its virtual time, as `tests/goto_api.rs`'s `Recorded` does, and a console built on
  it as `tests/live/support.rs` builds one. Write failing tests in a new
  `crates/finder/tests/mcp_session_tools.rs`:
  - `search_targets` for "orion" answers `{targets}` equal to
    `GET /api/targets?q=orion` (US2-S1);
  - `set_session` with `min_alt_deg` 85, and with `{"sun_guard": false}`, gets the
    route's 400 text as `isError: true`, with the session unchanged (US2-S2);
  - `get_guidance` from 350°/30° to 24°/42° gives clockwise 34° and up 12°, within 1e-9°
    (US2-S3);
  - the scope node's address and token are refused with `set_session`'s reason (US2-S4);
  - with the simulated mount connected at 40° N, 75° W, the site 40° N, 75° E is refused,
    and applied on a console with no mount (US2-S5);
  - each tool equals its route's JSON, the read-only hints are set, and `tools/list`
    names all three (US2-S6).

### Implementation

- [ ] T013 [US2] In `crates/finder/src/api.rs`, move `search_targets`'s body into
  `search`, which the route then calls (plan.md § The shared functions).
- [ ] T014 [US2] Add `set_session`, `search_targets` and `get_guidance` to
  `crates/finder/src/mcp/tools.rs`. `search_targets` calls `search` and answers
  `{"targets": [...]}`; `get_guidance` answers `{"guidance": ...}` from
  `session_response`.
- [ ] T015 [US2] In `set_session` in `crates/finder/src/mcp/tools.rs`, read the arguments
  as `set_target` does, refuse an edit that sets any other `SessionEdit` field with
  `set_session changes only site, min_alt_deg and target`, refuse any `site` key while the
  session says a mount is connected with
  `the site cannot change while a mount is connected`, and apply the rest with
  `edit_session`.
- [ ] T016 [US2] Run the `lint` and `test` gates. Confirm `tests/targets_api.rs` and
  `tests/session_node.rs` are unchanged and green. Commit the green story (FR-020;
  US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6).

## Phase 3: User Story 3 — Frames and the link (Priority: P1)

### Tests first

- [ ] T017 [US3] Write failing tests in a new `crates/finder/tests/mcp_frames.rs`:
  - `solve_frame` with `orion2.jpg` as base64 equals `POST /api/solve`, and
    `get_pointing` then returns the session's pointing (US3-S1);
  - two PGM images synthesised at run time and never committed, one of exactly
    `SOLVE_BODY_LIMIT` bytes (its header comment padded to the length) and one a byte
    longer: the first answered with a solve report of its width and height, the second
    refused with the size reason and the pointing unchanged (US3-S2);
  - on the simulator's live loop, `get_pointing` is within 1′ of the true eyepiece
    direction at mid-exposure (US3-S3);
  - `get_link` 2 s of virtual time after `set_unresponsive(true)` reads disconnected, with
    its `node` equal to the session's except `last_frame_age_s` (US3-S4);
  - `tools/list` names the three tools, with the read-only hints (US3-S5).

### Implementation

- [ ] T018 [US3] Add `base64 = "=0.23.1"` to `crates/finder/Cargo.toml` (owner question
  2). Update `Cargo.lock` by running cargo once without `--locked`, and confirm its diff
  is the one line in finder's dependency list.
- [ ] T019 [US3] In `crates/finder/src/api.rs`, move `solve`'s body after its form is read
  into `solve_upload`, which takes the solve gate's permit from its caller; the route
  still takes the permit before reading its form.
- [ ] T020 [US3] Add `solve_frame` to `crates/finder/src/mcp/tools.rs`: decode the base64
  with `base64`'s standard engine, refuse a decoded image over `SOLVE_BODY_LIMIT` with
  `the image is larger than POST /api/solve's 32 MiB limit`, and otherwise take the
  solve gate's permit and call `solve_upload` with the route's default options, so the
  latest pointing is set as the page sets it.
- [ ] T021 [US3] Add `get_pointing` and `get_link` to `crates/finder/src/mcp/tools.rs`,
  answering `{"pointing": ...}` and `{"node": ...}` from `session_response`.
- [ ] T022 [US3] Run the `lint` and `test` gates. Confirm `tests/api.rs` and
  `tests/link.rs` are unchanged and green, and that no file over 200 KiB is added. Commit
  the green story (FR-006, FR-021; US3-S1, US3-S2, US3-S3, US3-S4, US3-S5).

## Phase 4: User Story 4 — GoTo and Stop over MCP, and who moved the telescope (Priority: P1)

### Tests first

- [ ] T023 [US4] Write failing tests in a new `crates/finder/tests/mcp_motion.rs`, on the
  simulator in virtual time after a sync:
  - `describe_axes` and `get_axis_state` equal the simulator's (US4-S1);
  - `goto_target` completes within 1′ with no page action, with `motion.by` as `mcp`
    (US4-S2);
  - polled every 0.5 s: `motion` is `mcp` and `goto_target` until the end, then absent
    (US4-S3);
  - the page's `POST /api/goto` gives `motion` `page` and `goto`, and `POST /api/track`
    gives `page` and `track` (US4-S4; re-check when 015 lands);
  - a state file from before this feature loads with no `motion`, and never gains a
    `motion` key during or after an MCP GoTo (US4-S5).
- [ ] T024 [US4] Add failing stop tests to `crates/finder/tests/mcp_motion.rs`, each
  reading the recording mount's command log:
  - `stop_all` from a second client halts a GoTo within 0.1 s, with
    `the observer stopped it` and no correction for 5 s (US4-S6);
  - `stop` with `MCP-Protocol-Version: 1999-01-01` gets 400 and still stops (US4-S7);
  - a bare `stop_all`, with no `initialize`, `Accept` or protocol version, stops (US4-S8);
  - `stop` with no mount is a success, and both stops stay listed (US4-S9);
  - `stop_all` with a foreign `Origin` stops (US4-S10);
  - a batch of `stop_all` and `goto_target` with a foreign `Origin` gets 403, and with no
    `Origin` 415; each stops and starts no GoTo (US4-S11);
  - `goto_target` with `Host: evil.example:8080` gets 403, and no axis moves (US4-S12);
  - `get_session` and `goto_target` from `203.0.113.7` get 403, and no axis moves
    (US4-S13).

  Each "no axis moves" test first shows the mount moving on an accepted call.
- [ ] T025 [US4] Add failing tests to `crates/finder/tests/mcp_motion.rs`:
  - no tool result or call-log line holds the scope node's test token (US4-S14);
  - the captured call log has exactly two lines, for `goto_target` and then `stop_all`,
    in plan.md § The call log's form (US4-S15).

### Implementation

- [ ] T026 [US4] In `crates/finder/src/api.rs`, move the starts of `start_goto`,
  `second_sync` and 015's `POST /api/track` into `start_motion`, which records who started
  the motion before it starts and restores the record if the start is refused, and move
  `stop_mount`'s body into `stop`. The routes call them with `by = page` and `goto`,
  `second_sync` and `track` (re-check when 015 lands). No route's answer changes.
- [ ] T027 [US4] In `crates/finder/src/api.rs`, derive `motion` in `session_response`
  from that record and the motion's running state (plan.md § The session's `motion`), and
  add `AppState::with_call_log(..)`, stderr by default. Write nothing to `session.rs` or
  the session file.
- [ ] T028 [US4] Add `describe_axes`, `get_axis_state`, `goto_target`, `stop` and
  `stop_all` to `crates/finder/src/mcp/tools.rs`. `goto_target` calls `start_motion` with
  `by = mcp` and writes its call-log line. The stops answer `{"stopped": true}` without
  stopping again when the door's request extension says it has stopped.
- [ ] T029 [US4] In `crates/finder/src/mcp/door.rs`, recognise a `tools/call` of `stop`
  or `stop_all`, alone or in a JSON array, and carry it out once through `api::stop`
  before any check answers and before `rmcp` sees the request, writing the stop's
  call-log line and setting the extension. When a check fails, run nothing else
  (plan.md § The door).
- [ ] T030 [US4] Run the `lint` and `test` gates. Confirm `tests/goto_api.rs`,
  `tests/goto_sim.rs`, `tests/second_sync_sim.rs`, `tests/session_cli.rs` and 015's
  `tests/track_api.rs` are unchanged and green. Commit the green story (FR-007, FR-008,
  FR-012, FR-014, FR-017; US4-S1, US4-S2, US4-S3, US4-S4, US4-S5, US4-S6, US4-S7, US4-S8,
  US4-S9, US4-S10, US4-S11, US4-S12, US4-S13, US4-S14, US4-S15).

## Phase 5: User Story 5 — Direct axis moves, held to the page's guards (Priority: P1)

### Tests first

- [ ] T031 [US5] Write failing guard tests in a new
  `crates/finder/tests/mcp_axis_moves.rs`, on the simulator in virtual time, sampled every
  0.1 s:
  - with the Sun 5° up, all three moving tools are refused with `Sun is up` and nothing
    moves (US5-S1);
  - at dusk (Sun −10°, minimum 0°), a position on the Sun's azimuth at altitude 3° is
    refused with the planner's reason, and one 90° away is carried out (US5-S2);
  - a top-rate turn through the Sun's azimuth for 30 s from 40° away is refused (US5-S3);
  - a position below the minimum altitude is refused with its reason (US5-S4);
  - from 3° up with a minimum of 15°, 1°/s up for 20 s is carried out and stops at
    20.0 ± 0.1 s, and an azimuth turn from there is refused (US5-S5);
  - 1.2 turns from the first sync is refused, and 0.8 turns carried out (US5-S6);
  - with no sync, `move_axis` is refused with `the night has no sync yet` (US5-S7).
- [ ] T032 [US5] Add failing rate-move and limit tests to
  `crates/finder/tests/mcp_axis_moves.rs`:
  - 0.5°/s for 10 s stops at 10.0 ± 0.1 s, having moved 5.0° less the ramp, within 0.1°
    (US5-S8);
  - a 60 s duration is refused with the duration reason (US5-S9);
  - a 20 s move whose client goes away stops at 20.0 ± 0.1 s, with `heartbeats` growing
    and `last_heartbeat_at` never more than 1 s old (US5-S10);
  - sunrise during a rate move stops it within 1 s, with `axis_move` halted by the Sun
    guard (US5-S11);
  - ten moves within 1 s: one carried out, one `the mount is already moving`, eight
    refused for the rate limit, and `stop_all` stops (US5-S12);
  - `tools/list` describes both tools as moving a real telescope, and no result or log
    line holds the token (US5-S13);
  - the page's `POST /api/stop` halts a running move with `the observer stopped it`
    (US5-S14);
  - `goto_target` over a running move is refused with `the mount is already moving`
    (US5-S15).

### Implementation

- [ ] T033 [US5] In `crates/finder/src/goto.rs`, add `Planner::axis_path`,
  `Planner::rate_path`, `GoTo::check_axis_move`, `GoTo::recheck_axis_move`,
  `GoTo::halt_reason` and `GoTo::stops`, built from `ready`, `safe_path`,
  `check_request`, `prepare` and `guard()`'s test as plan.md § Decisions says. `guard()`
  calls `halt_reason`, and `prepare`'s shared steps move into a function both paths call.
  `Planner` gains no field, `Progress` gains no field, and no refusal or halt text
  changes (owner question 1).
- [ ] T034 [US5] Create `crates/finder/src/mcp/guard.rs` and add `pub mod guard;` to
  `crates/finder/src/mcp/mod.rs`: run a checked direct move through `start_motion`, set
  `axis_move`, command the mount, and watch it every 50 ms on the console's clock, sending
  stop all on Stop's generation, `halt_reason`, a refused re-forecast, the duration's end
  or arrival (plan.md § Decisions).
- [ ] T035 [US5] Create `crates/finder/src/mcp/limit.rs` and add `pub mod limit;` to
  `crates/finder/src/mcp/mod.rs`: one window sliding over the last second of the
  console's clock, accepting 2 moving calls across all clients, where a refused call
  doesn't count and an accepted one does.
- [ ] T036 [US5] In `crates/finder/src/api.rs`, report `axis_move` in `session_response`
  and derive `motion` for a running direct move (plan.md § The session's `axis_move`;
  owner question 3).
- [ ] T037 [US5] Add `goto_axis_position` and `move_axis` to
  `crates/finder/src/mcp/tools.rs`:
  - both go through `GoTo::check_axis_move` and `mcp::guard`, and write their call-log
    lines;
  - `move_axis` caps the rate at the top rate and refuses a duration outside 0 to 30 s
    with `duration_s must be more than 0 and at most 30`;
  - apply the limiter to them and to `goto_target`.
- [ ] T038 [US5] Run the `lint` and `test` gates. Confirm `tests/second_sync_sim.rs`,
  `tests/goto_api.rs`, `tests/goto_sim.rs` and `goto.rs`'s own tests are unchanged and
  green, and that the diff is under 64 KiB. Commit the green story (FR-009, FR-010,
  FR-011, FR-022; US5-S1, US5-S2, US5-S3, US5-S4, US5-S5, US5-S6, US5-S7, US5-S8, US5-S9,
  US5-S10, US5-S11, US5-S12, US5-S13, US5-S14, US5-S15).

## Phase 6: User Story 6 — Tracking over MCP (Priority: P2)

### Tests first

- [ ] T039 [US6] Write failing tests in a new `crates/finder/tests/mcp_tracking.rs`,
  feeding the console the simulator's true pointing plus noise, as 015's long tracking
  tests do (re-check when 015 lands):
  - `set_tracking` on M42 holds within 1′ over 10 min of virtual time, sampled every 1 s,
    with `get_session`'s `track.readout.rms_arcsec` equal to the route's and `motion`
    naming `set_tracking`, the Track-first GoTo included (US6-S1);
  - with no mount, it is refused with `POST /api/track`'s 409 reason, and no result or
    log line holds the token (US6-S2);
  - after `move_axis`, a second `set_tracking` on within 1 s is refused for the rate
    limit, and `set_tracking` off straight after is not (US6-S3; owner question 8).

### Implementation

- [ ] T040 [US6] Add `set_tracking` to `crates/finder/src/mcp/tools.rs`: `{"on": true}`
  calls `start_motion` with `by = mcp` and `set_tracking` and joins the limiter;
  `{"on": false}` calls the same function as `POST /api/track` for off, unlimited. Both
  write call-log lines. If 015's landed route needs `crates/finder/src/api.rs` to expose
  that function, US6 moves it there and changes no answer.
- [ ] T041 [US6] Run the `lint` and `test` gates. Confirm 015's existing tracking tests
  are unchanged and green, and that `crates/finder/src/tracking.rs` and
  `crates/finder/src/session.rs` are not in the diff. Commit the green story (FR-013;
  US6-S1, US6-S2, US6-S3).

## Phase 7: User Story 7 — The page says an MCP client is moving the telescope, and Setup shows the MCP address (Priority: P2)

### Tests first

- [ ] T042 [US7] Write failing Vitest tests:
  - in a new `crates/finder/web/src/components/McpNote.test.tsx`, rendered through
    `useSession` with a stubbed session: the note renders only while `motion.by` is
    `mcp`, naming the tool, and Stop shows while `motion` is present though the mount
    reports no movement;
  - in a new `crates/finder/web/src/components/McpSection.test.tsx`, a LAN answer lists
    every URL and `surface.local` with the WSL NAT note (US7-S2), a loopback answer lists
    only `localhost`, the off state is shown as off (US7-S3), and a health-shaped answer
    renders nothing.
- [ ] T043 [US7] Write `crates/finder/web/e2e/mcp.spec.ts` for the `simulate` project,
  with its own night pixel helpers:
  - it sets its own site, target and sync, as 014's `e2e/goto.spec.ts` reaches a running
    GoTo, then posts `goto_target` over HTTP: the note shows in the guidance block and top
    bar within 1 s, Stop is enabled, and Stop halts the GoTo, oracle the session's `goto`
    and `GET /api/sim/telescope` (US7-S1);
  - the night pixel check on the note and on Setup's MCP section, at 0 pixels (US7-S4).

### Implementation

- [ ] T044 [US7] Let `motion` reach the page: add `motion?` to `ObservingSession` in
  `crates/finder/web/src/api.ts`, copy it in `crates/finder/web/src/useSession.ts`, and
  pass it to `TopBar` in `crates/finder/web/src/App.tsx`.
- [ ] T045 [US7] Add `crates/finder/web/src/components/McpNote.tsx`, and place it in
  `crates/finder/web/src/components/GuidanceBlock.tsx` and
  `crates/finder/web/src/components/TopBar.tsx` beside the existing Stop. Give
  `mountMayMove` in `crates/finder/web/src/components/StopButton.tsx` an optional third
  argument, `motion`, and pass it from `TopBar.tsx` and
  `crates/finder/web/src/components/MountSection.tsx`.
- [ ] T046 [US7] Add `crates/finder/web/src/components/McpSection.tsx`, reading
  `GET /api/mcp` and rendering nothing for an answer that isn't an MCP status, and place
  it in `crates/finder/web/src/components/SetupPage.tsx` after the mount section.
- [ ] T047 [US7] In `crates/finder/web/playwright.config.ts`, add `mcp.spec.ts` to the
  `simulate` project's file match and to the `default` project's ignore list, re-reading
  the lines 014's and 015's US9 left, and change nothing else (owner approval 15).
- [ ] T048 [US7] Add the MCP section to `crates/finder/README.md` (US7-S5). Record the
  note and the Setup section as built in `DESIGN.md` and `.impeccable/design.json`
  (US7-S6).
- [ ] T049 [US7] Run the `lint` and `test` gates; the `test` gate runs the page's
  `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`, `npm test`,
  `npm run build`, `git status --porcelain -- dist | (! grep .)` and `CI=1 npm run e2e`,
  with every existing Vitest and Playwright test unchanged and green. Commit the rebuilt
  `crates/finder/web/dist`. Confirm `package.json` dependencies are unchanged. Commit the
  green story (FR-015, FR-018; US7-S1, US7-S2, US7-S3, US7-S4, US7-S5, US7-S6).

## Execution Order

The stories land in this order:

```text
US1 → US2 → US3 → US4 ─┬→ US5 → US6
                       └→ US7
```

Every arrow is a merge edge: the later story starts from a `main` that holds the earlier
one.
- **US2 to US6** each add tools to `crates/finder/src/mcp/tools.rs`, and shared functions
  to `crates/finder/src/api.rs`, so they run one after another.
- **US3** also changes `Cargo.lock` and `crates/finder/Cargo.toml` after US1. **US4**
  changes `mcp/door.rs` from US1. **US5** adds modules to `mcp/mod.rs` and entry points to
  014's `goto.rs`.
- **US7** may run beside US5 and US6, because it touches only `crates/finder/web/`,
  `crates/finder/README.md`, `DESIGN.md` and `.impeccable/design.json`.
- **`[P]`** marks nothing here: every task within a story shares a file with another.
- **Out of scope for every story:** `crates/math-core`, `proto/`, `crates/simulator`,
  `crates/scope-node`, the scope link's proto, `scripts/ci/`, `.github/`, `deny.toml` and
  existing tests.
