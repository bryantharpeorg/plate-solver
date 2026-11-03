# Implementation Plan: The Console's MCP Server

**Branch**: `016-mcp` | **Date**: 2026-10-28, refined 2026-11-01, 2026-11-02 and 2026-11-03 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Mount an MCP endpoint, `/mcp`, in the console's existing axum router, as `rmcp`'s
Streamable HTTP tower service, behind a door of the console's own. The door checks
`Origin`, `Host` and the peer address, and carries out any stop itself. Each tool is a
thin call into the function behind a console route or action, moved out of its private
handler where it lives today so that the route and the tool share it.

Every tool that moves the telescope runs the page's guards. Direct axis moves are checked
by 014's own planner and GoTo code, through public entry points added to `goto.rs`, and
run under a watcher that stops them in time. The session reports who started each motion,
and the page says so. No change reaches the scope node, its proto or the link, and
`crates/math-core` is untouched.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI. TypeScript with React 19 for the
page.

**Primary Dependencies**:
- already in `crates/finder`: axum 0.8.9, tokio, tower 0.5.2, serde, serde_json and clap;
- new: `rmcp` and `base64` (§ Dependencies);
- the page: Vite, Vitest and Playwright, with no new npm package.

**Storage**: nothing new is persisted. `motion` and `axis_move` are derived or held in
memory and never written to the session file.

**Testing**:
- Rust integration tests in new files under `crates/finder/tests/`, with a shared module
  `crates/finder/tests/mcp/support.rs`. They post JSON-RPC to `/mcp` through
  `router(..)` with `tower::ServiceExt::oneshot`, with peers set through axum's
  `MockConnectInfo` and the headers spec.md's shared setup names.
- The simulator runs as 013 sets it up: over an in-memory `tokio::io::duplex` transport,
  on a `VirtualClock`, under `#[tokio::test(start_paused = true)]`. The simulator keeps no
  command or heartbeat log, so the support module wraps `SimMount` in a recording mount,
  as 014's `tests/goto_api.rs` does with `Recorded`, and reads `LinkStatus` through
  `AppState::link()` (§ Plan traps, trap 15).
- Vitest and one Playwright file in the `simulate` project.

**Target Platform**: the `finder` binary in WSL2 on the Surface; clients on the Surface
(Claude Code, Claude Desktop) and on the phone.

**Constraints**:
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy and rustdoc, the 80% coverage floor,
  and the red-only night mode.
- `web/dist` is committed and must be the build of its source.
- No test moves real hardware or calls a real LLM.

**Scale/Scope**: one console, a handful of MCP clients on the house LAN, and 16 tools.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver or `crates/math-core`. `solve_frame` calls the same function as `POST /api/solve`. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml` or `CODEOWNERS` is edited. New tests go in new files. `crates/finder/tests/mcp/support.rs` is 016's own shared test module, created by US1; US2 to US6 may add new items to it, and may not change or remove any item an earlier story put there (approval 17). The owner's decisions are listed in spec § Owner approvals. |
| III. Tests first, independent expectations | Each read tool is compared with its route's JSON. Each moving tool is checked against the simulator's true eyepiece direction and axis state, and a recording mount's command log, in virtual time. |
| IV. Critical paths | One dependency change (`rmcp`, plus its tree in `Cargo.lock`, and `base64`, which that tree already holds), owner-approved. No `unsafe`, no proto change, no database-format change. |
| V. Standalone Rust, honest licensing | `rmcp` is Apache-2.0, and every package it adds is MIT, Apache-2.0 or both, inside `deny.toml`'s allow-list (§ Dependencies). |

## Dependencies

**`rmcp` = `=3.5.1`**, the official MCP Rust SDK:
- `default-features = false`, with features `server`, `macros` and
  `transport-streamable-http-server`, each of which 3.5.1 declares;
- it is Apache-2.0, as is `rmcp-macros`;
- its `rust-version` is 1.88, below this repository's 1.96.

3.5.1 (2026-10-05) is still the newest release at least two weeks old on 2026-11-01:
3.6.0 is dated 2026-10-20, 3.7.0 2026-10-22, and 3.8.0 is newer still. Releases are
frequent, so the pin is exact and moves only deliberately.

**`rmcp-macros` floats unless it is locked.** `rmcp` 3.5.1 asks for `rmcp-macros`
`^3.5.1`, so cargo picks the newest, 3.8.0. A scratch crate with finder's axum features
compiled under `-D warnings`, with clippy clean, against both 3.8.0 and 3.5.1. US1 locks
3.5.1 with `cargo update -p rmcp-macros --precise 3.5.1` (owner question 4).

**What it adds to `Cargo.lock`.** Resolved offline against `main`'s lock at 1c928e3
(unchanged at a43747f; at a02dc35 it differs only by d1dcddb's `pulldown-cmark` 0.13.5 to
0.13.6, a patch with the same dependencies, outside `rmcp`'s tree), in a scratch copy of
the workspace, adding `rmcp` adds 34 packages: `rmcp`, `rmcp-macros`, `schemars` and `schemars_derive` 1.2.3,
`serde_derive_internals`, `chrono` and its `iana-time-zone`, `iana-time-zone-haiku`
0.1.2 (its build dependency `cc` is already locked), `android_system_properties` and
`core-foundation-sys`, `uuid`,
`sse-stream`, `futures`, `futures-executor`, `futures-io`, `futures-macro`, `dyn-clone`,
`ref-cast` and `ref-cast-impl`, `darling` 0.24 with `darling_core`, `darling_macro`,
`ident_case` and `strsim`, `pastey` 0.2.3, `rand` 0.10.3, `rand_core` 0.10.1,
`chacha20`, `base64` 0.23.1, and five `windows-*` crates. Every one is MIT, Apache-2.0 or
both, and the highest `rust-version` among them is 1.89 (`uuid`). Cargo also:
- moves `futures-channel`, `futures-core`, `futures-sink`, `futures-task` and
  `futures-util` from 0.3.33 to 0.3.34, because it picks `futures` 0.3.34 for the new
  facade (a `--precise` walk can hold them at 0.3.33; owner question 4);
- points `tempfile` at `getrandom` 0.4.3 instead of 0.3.4;
- writes three existing dependency names with a version (`pastey 0.1.1`, `rand 0.9.5`,
  `base64 0.22.1`), because a second version of each is now locked.

The lock's diff is about 15 KB.

**Duplicate versions.** `cargo deny` will warn that `base64` (0.22.1, 0.23.1), `rand`
(0.9.5, 0.10.3), `rand_core` (0.6.4, 0.9.5, 0.10.1) and `pastey` (0.1.1, 0.2.3) are
locked twice. `deny.toml` sets `multiple-versions = "warn"`, so the check passes. The old
versions are held by `tonic` 0.14.6, `rav1e` and `av-scenechange`, which finder can't
move, so the warnings can't be settled without editing `deny.toml`, which Principle II
forbids. They are accepted under owner approval 13 (owner question 4).

**`base64` = `=0.23.1`** (US3, owner question 2): `solve_frame` decodes its image with the
version `rmcp` already locks, so it adds no package. Finder has no base64 decoder today,
and `rmcp` does not re-export its own. It is MIT OR Apache-2.0.

**Not checked here.** No RustSec advisory scan ran: the sandbox has no `cargo-deny`, and
CI's `deny` job checks advisories in the merge queue.

**Rejected:** writing JSON-RPC by hand. The SDK owns protocol negotiation, schemas and
transport details that change between revisions.

## Project Structure

```text
Cargo.lock                                # US1: rmcp and its tree; US3: base64
crates/finder/
├── Cargo.toml                            # US1: rmcp =3.5.1; US3: base64 =0.23.1
├── README.md                             # US7: the MCP section
├── src/
│   ├── lib.rs                            # US1: `pub mod mcp;`
│   ├── mcp/mod.rs                        # US1: McpConfig, the service and its mounting;
│   │                                     # US5: `pub mod guard; pub mod limit;`
│   ├── mcp/door.rs                       # US1: Origin/Host/peer; US4: stops carried out here
│   ├── mcp/tools.rs                      # US1 get_session, set_target; US2 session and catalog;
│   │                                     # US3 frames and link; US4 mount, GoTo, Stop;
│   │                                     # US5 direct moves; US6 set_tracking
│   ├── mcp/guard.rs                      # US5: direct-move runner and its watcher
│   ├── mcp/limit.rs                      # US5: the shared move limiter
│   ├── api.rs                            # US1–US6: the shared functions (§ Interfaces),
│   │                                     # the /mcp mount, GET /api/mcp, `motion`, `axis_move`
│   ├── goto.rs                           # US5: public entry points for direct moves
│   └── main.rs                           # US1: --no-mcp, connect info
├── tests/
│   ├── mcp/support.rs                    # US1 creates; US2–US6 only add (approval 17)
│   ├── mcp_endpoint.rs                   # US1
│   ├── mcp_session_tools.rs              # US2
│   ├── mcp_frames.rs                     # US3
│   ├── mcp_motion.rs                     # US4
│   ├── mcp_axis_moves.rs                 # US5
│   └── mcp_tracking.rs                   # US6
└── web/
    ├── src/components/McpNote.tsx (+ test)     # US7: the note in GuidanceBlock and TopBar
    ├── src/components/McpSection.tsx (+ test)  # US7: Setup's MCP section
    ├── src/components/GuidanceBlock.tsx, TopBar.tsx, SetupPage.tsx   # US7: placing them
    ├── src/components/StopButton.tsx, MountSection.tsx   # US7: Stop shown while `motion` is
    ├── src/api.ts, src/useSession.ts, src/App.tsx         # US7: `motion` reaches the page;
    │                                                      # App reads GET /api/mcp for Setup
    ├── dist/                                   # US7: rebuilt
    ├── playwright.config.ts                    # US7: mcp.spec.ts in two lists (approval 15)
    └── e2e/mcp.spec.ts                         # US7: in the `simulate` project
DESIGN.md, .impeccable/design.json              # US7
```

014's `goto.rs` gains public entry points in US5 and nothing else. 015's `tracking.rs`,
and the Track state 015 keeps in `session.rs` (`TrackRun` with its `armed` flag,
`Session::start_tracking`, `stop_tracking` and `track_live_pointing`), are not edited.

## Interfaces

### The endpoint

`/mcp` uses Streamable HTTP, mounted in the console's router as `rmcp`'s tower service
behind `mcp::door`:
- A POST carries one JSON-RPC message. Requests get one `application/json` answer;
  notifications and responses get 202. `rmcp` answers a JSON array (a batch, which MCP
  2025-06-18 removed) with 415, after the door has carried out any stop in it.
- A POST needs `Accept: application/json, text/event-stream` (406 otherwise) and
  `Content-Type: application/json` (415 otherwise), as `rmcp` 3.5.1 checks them.
- A GET gets 405 with `Allow: POST`, because the console sends nothing unasked.
- No `Mcp-Session-Id` is needed. All state is the observing session, which every client
  shares with the page. That works with the stateless 2026-07-28 revision, whose
  requests also carry the `Mcp-Method` and `Mcp-Name` headers `rmcp` checks, and with the
  older ones, where the session ID is optional.
- An unsupported `MCP-Protocol-Version` gets 400.

### `rmcp` settings

The defaults don't fit, so each is set explicitly. 3.5.1's `StreamableHttpServerConfig`
has a private field, so it is built with its builder calls, never a struct literal. These
calls compiled in a scratch crate under `-D warnings`, with clippy clean:

| Setting | Default | Set with | Why |
|---|---|---|---|
| `legacy_session_mode` | true | `.with_legacy_session_mode(false)` | No `Mcp-Session-Id` is needed on older revisions either. |
| `json_response` | false | `.with_json_response(true)` | Tools send no notification before their result, so every answer is `application/json`. |
| `allowed_hosts` | loopback names, matched exactly | `.disable_allowed_hosts()` | Exact loopback names would refuse the phone by IP. `rmcp` then accepts any well-formed `Host`, but still answers 400 for a missing or malformed one, before its method dispatch (`validate_dns_rebinding_headers`, first in `handle()`). The door runs first and makes the checks, including the stop exemption that `rmcp`'s checks would break. |
| `allowed_origins` | none | `.disable_allowed_origins()` | The door checks `Origin`. |
| `max_request_body_bytes` | 4 MiB | `.with_max_request_body_bytes(44 * 1024 * 1024)` | 4 MiB would cap `solve_frame` at a 3 MiB image. 44 MiB fits a 32 MiB image as base64 (4/3 of it, plus the JSON around it). |

The service is `StreamableHttpService::new(|| Ok(tools::Console::new(state.clone())),
Arc::new(NeverSessionManager::default()), config)`, mounted with
`Router::route_service("/mcp", ..)` behind
`axum::middleware::from_fn_with_state(state, door::door)`. The server's info is
`ServerConfig::new(ServerCapabilities::builder().enable_tools().build())` with
`Implementation::new("finder", env!("CARGO_PKG_VERSION"))`; 3.5.1's `ServerInfo` is
deprecated and fails under `-D warnings`. Tools are declared with `#[tool_router]`,
`#[tool]` and `#[tool_handler]`, and argument types derive `schemars::JsonSchema` through
`rmcp::schemars`.

### The door (`mcp::door`)

The door is axum middleware in front of `rmcp`. It runs before anything else, in this
order:
1. **Read the body**, up to 44 MiB, and answer 413 past that. The body is handed on
   unchanged.
2. **Carry out the stops.** If the body is a `tools/call` of `stop` or `stop_all`, alone
   or inside a JSON array, the door calls `api::stop` once for the request, before any
   check answers, and writes the call log's line for each stop.
3. **Origin.** An `Origin`, if present, must name the origin the request was sent to.
4. **Host.** The host in `Host`, and in `Origin` when present, must be an IP literal,
   `localhost`, or a name given with 012's `--allow-host` (owner question 7). As 012
   built it, the flag is repeatable and names hosts without a scheme or port; the host's
   name, without its port, must equal a listed name ignoring ASCII case, with no wildcard
   or suffix match, while any IP literal (IPv6 in brackets) and `localhost` pass
   unlisted. The list is `AppState`'s `allowed_hosts`, which `main.rs` fills with
   `with_allowed_hosts(args.allow_host)`, so the page's writes and `/mcp` accept the same
   names. A DNS-rebinding page sends the attacker's name in both `Origin` and `Host`, so
   check 3 alone passes it. Real MCP clients send no `Origin`, and reach the console by an
   address or a listed name. Checks 3 and 4 call the same functions as 012's same-origin
   check (§ The shared functions). A request with no `Host`, more than one, or one that
   is not a valid authority fails this check with 403. That rule is the door's own: 012's
   `SessionWrite` reads `Host` only when an `Origin` is present. From US4 on, any stop in
   such a request has already been carried out (step 2).
5. **Peer.** The peer address (`ConnectInfo<SocketAddr>`) must be loopback, RFC 1918,
   link-local (169.254/16, fe80::/10) or IPv6 unique local (fc00::/7). An IPv4-mapped
   IPv6 peer, as a dual-stack listener reports one, is judged as its IPv4 address. With no
   `ConnectInfo`, the request is refused. Under WSL's NAT, portproxy hides the real
   source, so the Windows firewall rule's scope does this job there. A phone that reaches
   the Surface by a global IPv6 address is refused.

**When a check fails.** The answer is 403 with a JSON-RPC error and no id, and nothing
else in the body runs, so a move can't ride in on a stop.

**Otherwise** the request goes on to `rmcp`, which builds the answer, with a request
extension that says the door has carried out its stops. The `stop` and `stop_all` tools
then answer `{"stopped": true}` without stopping or logging again. The answer may still
be an error, such as a 400 for an unsupported `MCP-Protocol-Version`, or a 406 for a
missing `Accept`. The stop has already been carried out.

**The binary serves with connect info.** `main.rs` serves with
`router(state).into_make_service_with_connect_info::<SocketAddr>()`, so the peer check
sees the real peer. In tests, `MockConnectInfo` sets the peer.

### The shared functions

Each tool calls the function behind its route. Today these bodies live inside private
handlers in `api.rs`, so the story that first passes one through moves its body into a
crate function that the route then calls too. No tool repeats a route's logic.

| Today | Shared function in `api.rs` | Story | Called by |
|---|---|---|---|
| `get_session`'s body: the snapshot, 014's `goto` progress and `goto_refusal` | `session_response(&AppState) -> SessionResponse` | US1 | `get_session`; `get_guidance`, `get_pointing` and `get_link` read their field from it |
| `put_session`'s rejection mapping (422 → 400, `body_text()`) and `Session::edit` | `session_edit_error(JsonRejection) -> ApiError` and `edit_session(&AppState, SessionEdit) -> Result<SessionView, ApiError>` | US1 | `set_target`, `set_session`, which read their arguments with `axum::Json::<SessionEdit>::from_bytes`; the route keeps its `Json` extractor, so its 415 for a missing content type stays |
| `SessionWrite`'s `Origin` and host-name rules | `check_origin(&Parts, &[String]) -> Result<(), ApiError>` and `host_allowed(&Parts, &[String]) -> bool`, each passed `AppState`'s one `allowed_hosts` list | US1 | `SessionWrite` (the first), the door (both) |
| `search_targets`'s body | `search(&AppState, q, limit) -> Result<Vec<SearchResult>, ApiError>` | US2 | `search_targets` |
| `solve`'s body after its form is read | `solve_upload(&AppState, permit, bytes, name, SolveOptions) -> Result<SolveReport, ApiError>` | US3 | `solve_frame`; the caller passes the solve gate's permit, so the route still takes it before reading its form |
| `start_goto`, `second_sync`, and the Track-on branch of 015's `set_track` (`POST /api/track`), which calls `Session::start_tracking(clock, link, goto)` after its two `no mount is connected` checks | `start_motion(&AppState, by, tool, start) -> Result<(), ApiError>` | US4 | `goto_target` (US4), the direct moves (US5), `set_tracking` (US6) |
| `stop_mount`'s body, which since 015's US7 first calls `Session::stop_tracking`: that clears `wanted` and the armed flag, sets `ended_reason` to `stopped` and stops the tracker | `stop(&AppState) -> serde_json::Value` | US4 | the door's stops; `set_tracking` off (US6), as `set_track`'s Track-off branch calls `stop_mount` |

The names are this plan's. A builder may adjust a signature to fit the code it finds, but
not inline a copy.

### The tool table

| Tool | Arguments | Passes through to | `structuredContent` | Story | Moves |
|---|---|---|---|---|---|
| `get_session` | none | `GET /api/session` | the session | US1 | no |
| `set_target` | `{target: string \| null}` | `PUT /api/session` with a target only; any other field is refused | the route's session view | US1 | no |
| `set_session` | `{site?, min_alt_deg?, target?}` | `PUT /api/session`, never the scope node | the route's session view | US2 | no |
| `search_targets` | `{q: string, limit?: integer}` | `GET /api/targets?q=` | `{targets: [...]}` | US2 | no |
| `get_guidance` | none | the session's `guidance` | `{guidance}` | US2 | no |
| `solve_frame` | `{image: base64 string, name?: string}` | `POST /api/solve`, held to its 32 MiB limit | the solve report | US3 | no |
| `get_pointing` | none | the session's `pointing` | `{pointing}` | US3 | no |
| `get_link` | none | the session's `node`: the link's health and the live loop's state | `{node}` | US3 | no |
| `describe_axes` | none | the mount call "describe the mount" (`MountClient::describe_mount`) | `{axes: [{axis, counts_per_revolution, top_rate_deg_per_s, can_goto}, ...]}` | US4 | no |
| `get_axis_state` | none | the mount call "axis state", each axis (`MountClient::axis_state`) | `{axes: [{axis, position, rate_deg_per_s, moving}, ...]}` | US4 | no |
| `goto_target` | none: the session's target, as on the page | `POST /api/goto` | `{started: true}` | US4 | yes |
| `stop` | none | `POST /api/stop` | `{stopped: true}` | US4 | stops |
| `stop_all` | none | `POST /api/stop`, which ends in the mount call "stop all" | `{stopped: true}` | US4 | stops |
| `goto_axis_position` | `{axis: "azimuth" \| "altitude", position: integer}` (counts) | the mount call "go to a position", after the direct-move check | `{started: true}` | US5 | yes |
| `move_axis` | `{axis, deg_per_s: number, duration_s: number}` | the mount call "move at a rate", after the direct-move check | `{started: true}` | US5 | yes |
| `set_tracking` | `{on: boolean}` | `POST /api/track` (`set_track`) | the session, as the route answers | US6 | yes |

**Arguments and results:**
- Arguments use the session's JSON names.
- The text content is the route's JSON exactly. `structuredContent` must be an object, so
  an object answer passes through, an array is wrapped under the tool's name for it, and
  a one-field read answers `{"<field>": <value>}`, where the value may be `null`.
- Every tool declares `outputSchema: {"type": "object"}` (owner question 9). Field-level
  schemas would need `JsonSchema` derives on the session's types in `session.rs`,
  `model.rs`, `goto.rs`, `solve.rs` and `targets.rs`, outside these stories' writes.
- A refusal is a tool result with `isError: true` and the route's own reason, from its
  400, 404, 409 or 413. JSON-RPC errors are kept for protocol faults.

**Annotations.** Read-only tools carry `readOnlyHint: true`. Each moving tool's
description says that it moves a real telescope. Annotations are hints that a client may
ignore, so no guard depends on them.

**`set_target`.** It reads its arguments as `PUT /api/session` reads its body, then
refuses an edit that sets any `SessionEdit` field but `target` with
`set_target changes only target`. Without that check, `{"target": "M42", "node": {...}}`
would parse and apply, and `set_target` would get around `set_session`'s whitelist and
its site lock.

**Both whitelists are exhaustive.** `set_target` and `set_session` each check their edit
with a destructure of `SessionEdit` that names every field and has no `..`, so a field a
later change adds to `SessionEdit` fails to compile until both tools decide on it. At
a02dc35 `SessionEdit` has six fields: `imaging_camera` (added by 015's US7), `site`,
`min_alt_deg`, `target`, `node` and `camera_settings`. `set_session` refuses
`imaging_camera`, `node` and `camera_settings`, as FR-020 limits it to the other three.

**`set_session`:**
- It reads its arguments as `PUT /api/session` reads its body, so an unknown field such as
  `sun_guard` gets the route's own 400 text. It then refuses an edit that sets any
  `SessionEdit` field but `site`, `min_alt_deg` and `target` with
  `set_session changes only site, min_alt_deg and target`. That covers the scope node's
  address and token: a client that could change them could point the console at a node
  of its own, or break its pairing. The token is never read back in any case.
- While the session says a mount is connected, it refuses any `site` key, set or cleared,
  with `the site cannot change while a mount is connected`. The Sun guard reckons the Sun
  from the site and the console's clock. A site moved by a few hours of longitude (75° E
  for 75° W) would make the Sun look set at noon, and the syncs already made would no
  longer fit. The clock can't be set outside tests and `--simulate`.

**`solve_frame`.** It decodes the base64 with `base64` 0.23.1's standard engine, refuses a
decoded image larger than `SOLVE_BODY_LIMIT` with
`the image is larger than POST /api/solve's 32 MiB limit`, and otherwise takes the solve
gate's permit and calls `solve_upload` with the route's default options. The route's own
413 text names its form, so the tool's reason is its own.

**`stop_all`** goes through the console's Stop rather than straight to the link, so a GoTo,
a direct move or tracking in progress ends too. A bare stop all on the link would be
undone by the GoTo's next correction.

### `GET /api/mcp`

The route is read-only:

```text
{ enabled: bool, local_only: bool, urls: ["http://localhost:8080/mcp", ...],
  allowed_hosts: ["surface.local", ...] }
```

**`urls`.** Under a loopback listen address, `urls` is only `http://localhost:<port>/mcp`,
and `local_only` is true. Under `0.0.0.0` or `::`, it is `localhost` plus one URL for each
non-loopback address the host reports, with IPv4 and link-local addresses first. Global
IPv6 addresses are left out, because the door refuses them. `<port>` is the port the
console bound, which `main.rs` records after binding. With `--no-mcp`, `enabled` is false.
`allowed_hosts` is `AppState`'s `allowed_hosts`, the names given with `--allow-host`, as
given (owner question 7).

**Where the addresses come from** (owner question 6). Rust's standard library cannot list
a host's interfaces, and finder has no crate that does. The addresses come through a small
trait that tests replace with a list of their own. In production it reports the address
the host would send from: a UDP socket bound to the unspecified address and connected to
`192.0.2.1:9` (TEST-NET-1), whose `local_addr()` names the outward interface. Connecting
a UDP socket sends nothing. Under WSL's NAT that address is the WSL machine's, which the
phone cannot reach, so Setup says so and names the README's section.

The route is the one interface this trio adds beyond the tool table and the session's two
new fields (owner approval 14).

### The session's `motion`

The session gains `motion: {by: "page" | "mcp", tool, since}`:
- `tool` is the page's action (`goto`, `second_sync`, `track`) or the MCP tool's name.
- It is derived on each session read in `session_response`, and is present only while
  the motion it names runs: a GoTo while 014's progress is `slewing`, `solving` or
  `correcting`; a direct move while `axis_move` is `moving`; Track while 015's
  `track.state` is `armed`, `tracking` or `keyhole_wait`, including the GoTo Track starts
  first. Those are the names 015 serializes: `TrackView.state` is `armed` while Track
  waits on a GoTo, and otherwise the tracker's `off`, `tracking` or `keyhole_wait`, which
  covers the keyhole slew and exit solve (`tracking::State`).
- Every start goes through `start_motion`, which records who started it, which tool and
  when, before the motion starts, and puts the previous record back if the start is
  refused. One lock covers the record and the start, so a read never pairs a new motion
  with an old source.
- It is never written to the session file: `StoredSession` refuses unknown fields, and
  `tests/session_cli.rs` pins the file's three durable keys.

### The session's `axis_move`

`axis_move: {tool, state: "moving" | "done" | "halted", reason?}` reports the latest direct
axis move (owner question 3). It is left out until the first one, kept until the next, as
014 keeps `goto` until the next GoTo, and never written to the session file. `reason` is
the text of whatever halted the move: 014's `the observer stopped it`,
`halted by a lost link` or `halted by the Sun guard`, or the planner's reason when a
re-forecast fails.

### The call log

Every moving or stopping MCP call is written as one line:
`mcp <tool> <arguments as compact JSON> -> <structuredContent as compact JSON>`, or
`-> error: <reason>` for a refusal. The writer is held by `AppState`: stderr by default,
and a shared buffer in tests, set with `AppState::with_call_log(..)`. The door writes the
stops' lines and `mcp::tools` the moves'. No line holds the scope node's token.

## Shapes that existing tests pin

These are not changed by any story:
- **Rust signatures:** `AppState::new`, `AppState::shared` and `router`. MCP is added with
  a builder, `AppState::with_mcp(McpConfig)`. MCP is on by default, and its host names
  are 012's `with_allowed_hosts` list, which `with_mcp` leaves as it is. 014's
  `GoTo::start`, `start_target`, `second_sync`, `stop`, `progress` and `refusal` keep
  their signatures (`tests/goto_api.rs` and `tests/goto_sim.rs` call them).
- **014's planner and progress:** `Planner` gains methods and no field, because
  `tests/second_sync_sim.rs` builds a `Planner` literal (lines 91–106), and so does
  015's `tracking.rs` for its keyhole slew (`Shared::keyhole`, lines 553–561).
  `Progress`, which the session serializes as `goto`, gains no field, because
  `web/src/test/fixtures/session-goto-m42.json` and the page's tests read its shape.
  Every refusal and halt text in `goto.rs` stays byte-for-byte.
- **Existing routes:** `POST /api/stop`'s exemption from the same-origin check, its
  answer, and every `/api/*` answer, including `PUT /api/session`'s 415 for a body that is
  not JSON and `POST /api/solve`'s 413. Tools read those answers and never reshape them.
- **The session:** existing session file fields and the session JSON, which `motion` and
  `axis_move` only add to, each left out until it has a value.
- **The page:** the page's existing components and tests, re-read at a02dc35, with 015's
  US8 and US9 in place.
  - The note is a new component placed inside GuidanceBlock and TopBar. GuidanceBlock has
    no Stop (it renders `GoToControl` and 015's `TrackControl`, whose only button is the
    `Track` switch), and TopBar renders the page's only `StopButton` (`TopBar.tsx:112`).
    No second Stop is added: `e2e/goto.spec.ts:72` finds Stop with a strict
    `getByRole("button", { name: "Stop", exact: true })`, and `StopButton.test.tsx:80`
    finds it unscoped.
  - 015's US8 gave `mountMayMove` a third argument, `track`, which `TopBar` passes through
    `StopButton`'s `track` prop and `MountSection` does not pass
    (`mountMayMove(mount, session.goto)`). `StopTrack.test.tsx` pins Stop in the top bar
    while `track.state` is `armed`, `tracking` or `keyhole_wait`, on `/` and `/setup`.
  - Setup's section is a new component that `SetupPage` renders only when `App` passes it
    the `GET /api/mcp` status as an optional prop. `App` reads that route only while the
    path is `/setup`. So `SetupPage` itself makes no new request. That matters because
    `ScopeNodeSection.test.tsx`, `AimTap.test.tsx`, `MountSection.test.tsx`,
    `FieldWarning.test.tsx`, `SetupPage.test.tsx`'s `open()` and 015's
    `ImagingCameraSection.test.tsx`'s `setup()` render `SetupPage` directly, and some of
    them count its fetches exactly (`ScopeNodeSection.test.tsx:81` and `:134`;
    `AimTap.test.tsx:83`, `:113`, `:123` and `:134`).
  - The App-level cases that reach `/setup` (`SetupPage.test.tsx:118–163` and
    `:165–184`, `StopButton.test.tsx`'s `/setup` cases, and 015's `StopTrack.test.tsx`
    `/setup` cases and `ImagingCameraSection.test.tsx`'s App case) answer URLs they don't
    name with a health-shaped body or `{stopped: true}`, and assert with
    `toHaveBeenCalledWith`, on `pushState` or on what they render, never on a count of
    fetches. So `McpSection` renders nothing for an answer that isn't an MCP status, and
    `App` catches every failure of its read: a rejected fetch, a non-OK answer or a body
    that isn't JSON leaves the prop unset.
  - Neither new view renders an element with role `status` or `alert`.
    `e2e/session.spec.ts:117` and `e2e/simulate.spec.ts:318` find Setup's save messages
    with an unscoped `page.getByRole("status")`, `e2e/goto.spec.ts:193` finds the
    guidance block's GoTo status with `block.getByRole("status")`, and
    `e2e/tracking.spec.ts:72` finds the `Track` region's status inside the block; all are
    strict.
  - `useSession.ts` passes `motion` through only when present, as it passes 014's
    `goto_refusal`, so `useSession.test.ts`'s `toEqual` on a health body still holds.
- **Playwright:** `playwright.config.ts`, its servers and its existing projects. The new
  `crates/finder/web/e2e/mcp.spec.ts` runs in the `simulate` project, against its
  `finder --simulate` server and fresh `--state`.
  - At a02dc35, after 015's US9, the `default` project spans lines 34–37: `{`,
    `name: "default",`, its `testIgnore` of `session.spec.ts`, `simulate.spec.ts`,
    `goto.spec.ts` and `tracking.spec.ts` on one 106-character line (line 36), and `},`.
    The `simulate` project (lines 45–51) has
    `testMatch: ["**/simulate.spec.ts", "**/goto.spec.ts", "**/tracking.spec.ts"]` on one
    83-character line (line 47), with `workers: 1`.
  - US7 appends `"**/mcp.spec.ts"` to both lists, laid out as Prettier formats them, and
    changes nothing else (owner approval 15). `.prettierrc.json` sets `printWidth` 120,
    and the `test` gate's `npm run format:check` covers the config. The ignore line would
    reach 124 characters, so Prettier puts `testIgnore`'s five entries one to a line,
    between `testIgnore: [` and `],`. The match line reaches 101 characters and stays one
    line. (Checked on a scratch copy with Prettier 3.7.4; the page asks for `^3.9.10`.)
  - Any other config change escalates.

## Decisions and the alternatives rejected

### Where MCP lives

**One endpoint, on the console's own port.**
- *Rejected: a second port for MCP.* Under WSL2 every port needs its own portproxy or
  Hyper-V firewall rule, and two ports double what can go wrong at the telescope.
- *Rejected: stdio.* The phone can't launch a process in WSL.

**Reaching it under WSL2, as the page is reached.** `/mcp` is served wherever the page is,
so it reaches the LAN only when the page does:
- The default `--listen 127.0.0.1:8080` keeps MCP on the Surface, and
  `--listen 0.0.0.0:8080` opens both. MCP adds no flag that widens the listen address.
- On the Surface, Claude Code in WSL and Claude Desktop on Windows reach
  `http://localhost:<port>/mcp`. NAT mode forwards Windows' localhost to WSL, and mirrored
  mode shares it.
- The phone reaches `http://<surface>:<port>/mcp` through whatever the owner set up for
  the page: mirrored mode with a Hyper-V firewall rule, or `netsh interface portproxy`
  with a Windows firewall rule under NAT. Either keeps the client's own `Host`.
- Under NAT, the WSL address the console sees is not the Surface's LAN address. Setup says
  so and names the README's section.

### Who may call it

**Open on the LAN, with `Origin`, `Host` and the peer checked.** The MCP specification
says servers MUST validate `Origin`, SHOULD bind to localhost and SHOULD authenticate. The
console keeps the MUST and binds to localhost by default. By the owner's decision it asks
for no token.
- *Rejected: a token, as the scope link has.* The owner rejected it, because clients on
  the house LAN are trusted. A token can be added later without changing the tools.

**One host rule, one list.** The door's `Host` rule is 012's: an IP literal, `localhost`
or a name given with `--allow-host`. It checks it on every request, where 012's
same-origin check skips it when there is no `Origin`. Both read `AppState`'s one
`allowed_hosts` list, so a name the owner lists for the page's writes also opens `/mcp`,
and no name opens one without the other (owner approval 3; owner question 7, answered
2026-11-03).
- *Rejected: a second flag, `--mcp-allow-host`, either as `/mcp`'s only list or adding
  names on top of `--allow-host`.* 012's flag already lists the names the console
  trusts, under the same rule.

**Stop is exempt, and carried out by the door.** A stop must never be refused. Carrying it
out in the console's own layer means nothing in the SDK can stand in front of it: not its
`Accept` or protocol-version checks, not a missing `initialize`, not its refusal of
batches, not a future default.

### What the tools do

**Passthrough means the same functions.** Each tool calls the function behind its route
(§ The shared functions), so the answers can't drift from the page's.

**No confirmation.** A moving tool starts the motion before it answers. The page shows the
note, and Stop stays on every page. The MCP specification's advice to keep a human in the
loop is left to the client, which may ask its own user.

**Safety is not passed through.** Per ADR-0001, the console owns pointing, and the Sun
guard lives in the console. The scope node only drives axes, and can't tell where they
point on the sky. A raw mount call passed straight to the link would skip the Sun guard,
and a 16-inch aperture pointed at the Sun is a fire and eye hazard. So every moving tool
runs the same guards as the page's actions:
- the Sun guard on motion: no motion with the Sun's centre above −0.83°, and no path
  within 15° of the Sun;
- the minimum altitude, never below 0°, at the end of the move and along its path;
- 014's path checks: the one-turn cable rule (azimuth kept within one turn either way of
  its position at the night's first sync), and any range limit the mount adapter reports;
- Stop wins, on the console and on the node, and the node's 1 s watchdog;
- tracking's own ends: sunrise, and the target sinking below the minimum altitude.

*Rejected: passing the raw calls through and trusting the node's watchdog.* The watchdog
catches a silent console, not a bad direction.

**Direct axis moves are checked by 014's code, not a copy of it.** The page has none, so
US5 gives 014's planner and GoTo owner the public entry points these moves need, built
from the code GoTo already runs:
- `Planner::axis_path(destination)` runs the planner's `ready` (a capable mount, `Sun is
  up`), refuses a destination the model puts below the lower of the start's altitude and
  the minimum altitude with `the position is below the minimum altitude`, and checks the
  one-axis path to it with `safe_path`, answering the planner's own
  `no path keeps clear of the Sun guard, minimum altitude and cable wind` when it does not
  keep clear.
- `Planner::rate_path(axis, deg_per_s, duration_s)` does the same for the position where
  the axis would come to rest: the rate, capped at the axis's top rate, for the duration,
  plus the stopping distance at the planner's `ACCELERATION`.
- `GoTo::check_axis_move(request)` runs `check_request`'s and `prepare`'s steps for the
  request (`a GoTo is already running`, `no mount is connected`, `mount did not answer`,
  `the night has no sync yet`, `a site is needed`, `Sun is up`,
  `the mount is already moving`, the observed cable wind), builds the same `Planner`, and
  calls `axis_path` or `rate_path`. `GoTo::recheck_axis_move` repeats it from the moving
  axis for a running move's remaining path.
- `GoTo::halt_reason()` is `guard()`'s test, `halted by a lost link` or
  `halted by the Sun guard`, which `guard()` then calls itself.
- `GoTo::stops()` is a receiver on the Stop generation that `GoTo::stop` advances.

So a sync is needed, as approval 5 says: without the pointing model, the console can't
turn axis positions into a sky direction, so it can't check the Sun. And a telescope
below the minimum altitude may only raise its altitude axis, with no azimuth turn, until it
clears the minimum, because that is what `safe_path` allows a GoTo from low down (owner
question 1).

**Running a direct move (`mcp::guard`).** After the check passes, `mcp::guard` records the
motion through `start_motion`, sets `axis_move` to `moving`, and commands the mount: a
go-to for `goto_axis_position`, a rate for `move_axis`. A watcher then runs on the
console's clock every 50 ms, 014's `TICK`, and sends stop all, as 014's halts do, at the
first of:
- Stop's generation moving on (`the observer stopped it`);
- `GoTo::halt_reason()`;
- `GoTo::recheck_axis_move` refusing the remaining path, such as after the minimum
  altitude is raised;
- the duration's end (`done`), or the axis arriving (`done`).

While a direct move runs, a GoTo or another direct move is refused by 014's own
`the mount is already moving`, and a direct move during a GoTo by
`a GoTo is already running`.

**Track and the MCP moves** (owner question 10). 014's two refusals don't cover Track. As
015 landed, its tracker drives its keyhole slew itself, with the mount call `goto_axis`
over the link rather than through 014's GoTo owner, then waits at the exit with both axes
at rest and no GoTo running (`Shared::keyhole` in `tracking.rs`), so neither refusal fires
then; `goto.rs` has no Track check at all. And `Session::start_tracking` refuses Track on
only for 015's FR-013 reasons (no mount, no sync, Sun up, a target that isn't pointable;
in the code also no site and no target) and for a refusal of the GoTo it starts. A
telescope already within 1′ of the target starts tracking at once, with no GoTo, so
nothing refuses Track on top of a running direct move. So, on the MCP path only:
- `goto_target`, `goto_axis_position` and `move_axis` are refused with `Track is on`
  while the session's `track.state` is anything but `off` (`armed`, `tracking` or
  `keyhole_wait`; a session with no `track` counts as off), after the limiter and before
  014's checks;
- `set_tracking` with `on: true` is refused with `a direct axis move is running` while
  `axis_move` is `moving`, after the limiter and before `POST /api/track`'s function;
- Track turned on during a GoTo still arms, as 015's FR-014 says.

The page's `POST /api/goto` and `POST /api/track` keep exactly the answers 015's tests
pin. What 016 relies on in 015, as landed: `track.state` is serialized in the session
under those stable names (`TrackView.state`, documented in `api.rs`'s module docs as
`armed` or the tracker's stable state name), and 015 defines nothing for a GoTo or a
direct move requested during `keyhole_wait`.

**A rate limit on moves (`mcp::limit`).** The tools page says servers MUST rate limit tool
invocations:
- Moving tools share one limit of 2 calls a second across all clients: `goto_target`,
  `goto_axis_position`, `move_axis` and `set_tracking` with `on: true`.
- The window slides over the last second of the console's clock. A call over the limit
  is refused with `the rate limit of 2 moving calls a second was reached`, moves nothing,
  and does not count against later windows. A call the limit accepts counts, even if a
  guard then refuses it.
- Reads, `stop`, `stop_all` and `set_tracking` with `on: false`, which only ends motion,
  are never limited (owner question 8).

This stops a looping client from flooding the scope link's queue, and leaves Stop
untouched.

### Who moved it

The session's `motion` shows the source. The page's note reads "Moved by an MCP client ·
<tool>" while `by` is `mcp`, and the top bar's Stop is shown whenever `motion` is present.
Every moving or stopping MCP call is also written to the call log with its tool, arguments
and result. The scope node's token is never in a log line or a result.

### How it is tested

**Tests go through the router.** The Rust tests need no MCP client crate and no socket.
Each read tool is compared exactly with its route's answer for the same input. Each moving
tool runs against the simulator in virtual time, with the recording mount's command log
and the console's `LinkStatus` as oracles.

## User Story 1 — The endpoint and its door

**Dependency.** Add `rmcp = { version = "=3.5.1", default-features = false, features =
["server", "macros", "transport-streamable-http-server"] }` to `crates/finder/Cargo.toml`.
Run cargo once without `--locked`, then `cargo update -p rmcp-macros --precise 3.5.1`
(owner question 4), and compare the lock's diff with § Dependencies. Don't edit
`deny.toml`.

**The service.** `mcp::mod` builds the service with the settings in § `rmcp` settings.
`api.rs` mounts it at `/mcp` behind `mcp::door` unless MCP is off. With MCP off, `/mcp`
falls through to the router's page fallback, which answers a POST with 405, as it answers
`/no-such-path`.

**Flags and the first tools.**
- `main.rs` gains `--no-mcp`, builds `AppState::with_mcp(..)`, records the bound address
  after binding, and serves with connect info. It adds no host flag: the door reads the
  `--allow-host` names that `main.rs` already passes to `with_allowed_hosts` (owner
  question 7). The text that says those names are for same-origin writes, in
  `--allow-host`'s help line, `with_allowed_hosts`'s doc comment and the crate docs in
  `lib.rs`, gains `/mcp`, so the flag says what it now opens.
- `api.rs` gains `GET /api/mcp` and the US1 shared functions: `session_response`,
  `session_edit_error`, `edit_session`, `check_origin` and `host_allowed`.
- `get_session` and `set_target` are the first two tools, in `mcp::tools`; `set_target`
  refuses every field but `target` (§ The tool table).
- `tests/mcp/support.rs` holds the request helper with spec.md's shared headers,
  `Host: localhost:8080` included, each overridable or removable per test. Later stories
  only add to the module (approval 17).

## User Story 2 — Session and catalog tools

`set_session`, `search_targets` and `get_guidance` go in `mcp::tools`. `search_targets`
calls `search`, which US2 moves out of the route. `set_session` reads its arguments with
`axum::Json::<SessionEdit>::from_bytes` and `session_edit_error`, refuses the other
`SessionEdit` fields and a site change while a mount is connected (§ The tool table), and
applies the rest with `edit_session`. The support module gains the simulator rig with the
recording mount, built as 014's `tests/goto_api.rs` builds its `Rig`: the live loop with
`with_live_solver` over the centroid solver, 0.5 s frames, and the aim point at
(700, 420).

`get_guidance`'s test follows 012's `tests/pushto_api.rs`: a solved `orion2.jpg` pointing
and M78 at 40° N, 75° W on `FixedClock`, compared with `pushto::turn_tilt` of the two
`sky::alt_az` positions. The session takes a pointing only as RA/Dec and a target only
as a catalog id, so a Given of exact azimuths and altitudes would need inverse precession
that the crate doesn't have.

## User Story 3 — Frames and the link

**`solve_frame`.** Add `base64 = "=0.23.1"` to `crates/finder/Cargo.toml` (owner question
2); the lock gains only the dependency's line in finder's entry. US3 moves `solve`'s body
after its form into `solve_upload`, and the tool calls it as § The tool table says, so the
latest pointing is set exactly as the page sets it. The test synthesises its 32 MiB PGM
images at run time, padding the header's comment to the exact length.

**`get_pointing` and `get_link`.** `get_pointing` reads the session's `pointing`, which is
live while 013's loop runs. `get_link` reads the session's `node`.

## User Story 4 — GoTo and Stop over MCP, and who moved it

**Reads and moves.** `describe_axes` and `get_axis_state` go over the scope link's mount
calls. US4 moves the GoTo, second-sync and Track starts into `start_motion`, which
`POST /api/goto` calls with `by = page, tool = goto`, `POST /api/model/second-sync` with
`second_sync`, 015's `POST /api/track` (`set_track`'s Track-on branch, around its
`Session::start_tracking` call) with `track`, and `goto_target` with
`by = mcp, tool = goto_target`.

**`motion`.** `session_response` derives it as § The session's `motion` says. Nothing is
added to `session.rs` or the session file.

**Stop.** US4 moves `stop_mount`'s body, `Session::stop_tracking` first as 015's US7 left
it, into `stop`, which `POST /api/stop` and `set_track`'s Track-off branch keep calling.
`mcp::door` recognises and carries out stops before `rmcp`, as § The door says, including
for batches and for requests that fail a check: a foreign `Origin`, a rebinding or
missing `Host`, a peer outside the LAN, or no connect info. The `stop` and `stop_all`
tools answer without stopping again when the door's extension says it has.

**The log and the token.** Add `AppState::with_call_log(..)` and the line format of § The
call log. Add the token-never-leaves test.

**Spacing GoTos in the tests.** US5 adds a limit of 2 moving calls in any sliding second
to `goto_target`, and US5 may not edit `mcp_motion.rs`. So within one rig, US4's tests
keep `goto_target` calls at least 1 s of virtual time apart (§ Plan traps, trap 26).

## User Story 5 — Direct axis moves and the rate limit

**014's entry points.** Add `Planner::axis_path`, `Planner::rate_path`,
`GoTo::check_axis_move`, `GoTo::recheck_axis_move`, `GoTo::halt_reason` and `GoTo::stops`
to `goto.rs`, as § Decisions says. `guard()` calls `halt_reason`, and `prepare`'s shared
steps move into a function both paths call, so no check is written twice. `Planner` gains
no field, and no refusal text changes.

**`goto_axis_position` and `move_axis`.** Add them in `mcp::tools`, with `mcp::guard`
running each move and its watcher. `move_axis` refuses a duration outside 0 to 30 s with
`duration_s must be more than 0 and at most 30` and caps the rate at the axis's top rate.

**`axis_move`.** `session_response` reports it (§ The session's `axis_move`).

**The limiter.** Add `mcp::limit` and apply it to `goto_target` and the two direct moves.

**Track.** After the limiter, the three moving tools refuse with `Track is on` while
015's `track.state` is anything but `off` (§ Decisions, "Track and the MCP moves"; owner
question 10). US5-S16 turns Track on through the page's `POST /api/track`, because
`set_tracking` arrives in US6.

## User Story 6 — Tracking over MCP

**`set_tracking`.** It answers the session, as `set_track` does. For `{"on": true}` it
calls `start_motion` with `by = mcp, tool = set_tracking` around the same Track start
`set_track` makes, so it refuses with the route's 409 reasons. For `{"on": false}` it
calls US4's `stop`, as `set_track` calls `stop_mount` for off, so turning Track off over
MCP ends every motion, as Stop does, and is never refused. `{"on": true}` joins the move
limiter, and is refused with `a direct axis move is running` while `axis_move` is
`moving` (owner question 10). If US4 left the Track start private to `set_track`, US6
moves it into a crate function in `api.rs` and changes no answer.

**`get_session`.** It already includes tracking's readout, because it passes the session
through.

**The feed.** US6's tests drive the console's own live loop: US2's rig, whose
`with_live_solver` over the centroid solver solves the simulator's frames every 0.5 s.
As 015's US7 landed, `live.rs` calls `Session::track_live_pointing` after each live
pointing it records, and that hands the aim point's error to the console's tracker
whenever the tracker's state is not `off`. So those solves reach the tracker with no hook
of 016's. 015's `tests/track_api.rs` drives the same rig, 014's `goto_api.rs` `Rig`, for
its readout. What reading cannot settle is whether a 10-minute paused-time run of the
live loop at 0.5 s frames fits the gates' and the coverage job's time: 015's hour-long
`tests/tracking_night.rs` feeds `Tracker::pointing` directly, not through live solves.
US6 measures it while developing. If it doesn't fit, US6 escalates, because a shorter
span in US6-S1 is a criteria change for the owner, and a hook is not the fix.

## User Story 7 — The page and the README

**`motion` reaches the page.** `api.ts`'s `ObservingSession` gains `motion?`;
`useSession.ts`, which copies the session field by field, copies it only when present;
`App.tsx` passes it to `TopBar`.

**The note.** `McpNote` renders "Moved by an MCP client · <tool>" from the session's
`motion` while `by` is `mcp`. It is placed in GuidanceBlock, which has no Stop and gets
none, and in TopBar beside its single `StopButton`. No second Stop is rendered anywhere
(§ Shapes that existing tests pin, "The page"). `StopButton`'s `mountMayMove` also
returns true while `motion` is present, through an optional fourth argument after the
`track` that 015's US8 added as its third. `StopButton` gains an optional `motion` prop
beside `track`, which TopBar passes; MountSection passes `undefined` for `track`, as it
passes none today, and the session's `motion`. So Stop shows from the moment a motion
starts, before mount telemetry reports moving axes. Because `motion` is also present
while Track is armed, tracking or at the keyhole, MountSection then counts the mount as
moving, where 015 left it idle unless telemetry reports movement. The note is plain
text, with no `status` or `alert` role.

**Setup's MCP section.** `App.tsx` reads `GET /api/mcp` each time the path becomes
`/setup`, catches every failure, and passes the answer to `SetupPage` as an optional
prop. `SetupPage` renders `McpSection` last, after every existing section (at a02dc35 the
last is `AimTap`, after 015's `ImagingCameraSection` and `FieldWarning`), only when it
has the prop. The answer arrives after Setup has drawn, so a section placed last moves
nothing that `e2e/simulate.spec.ts`'s Setup tap measures (`:309–316`).

`McpSection` takes the status as a prop and fetches nothing. It lists the URLs, the
allowed host names and the off or local-only notes, including the WSL NAT note naming the
README's section, as plain text with no `status` or `alert` role. A status without a
boolean `enabled` and an array `urls` renders nothing. `McpSection.test.tsx` renders
`McpSection` directly with each status.

**Records, tests and the build.**
- The README gains its MCP section; `DESIGN.md` and `.impeccable/design.json` record both
  views as built.
- `crates/finder/web/e2e/mcp.spec.ts` covers the GoTo flow and the night pixel check of
  both views. It reaches a running GoTo as 014's `e2e/goto.spec.ts` does, and carries its
  own night pixel helpers, because `e2e/session.spec.ts`'s and `e2e/goto.spec.ts`'s are
  local to those files.
- With `workers: 1`, Playwright runs the `simulate` project's files in name order: goto,
  mcp, simulate, then 015's tracking. So `mcp.spec.ts`'s `afterAll` leaves the server as
  `goto.spec.ts`'s does (`goto.spec.ts:109–116`): it posts `/api/stop`, sets frames
  running, deletes `/api/aim` and resets the night's model, because `simulate.spec.ts`
  runs next and its `beforeAll` asserts there is no aim point.
- `playwright.config.ts` gains `mcp.spec.ts` in its two lists, laid out as Prettier
  formats them (owner approval 15).
- `web/dist` is rebuilt.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at a02dc35; the commands are unchanged since
  1c928e3, and 8138f69 changed only the linker arguments the gates export in
  `RUSTFLAGS`, beside `-D warnings`):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, Rust and page stories alike, and the judge sees their results.
- **CI in the merge queue** runs every `ci.yml` job. Some have no factory counterpart:
  coverage at 80%, the cedar e2e eval and canary, `cargo machete`, `cargo deny`, the
  secret scan, and the build on Rust 1.96 (`msrv`). The sandbox has neither cargo tool
  and builds on stable, so a story meets those checks by construction (§ Plan traps,
  trap 5). A story is not done until it would pass them.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `012-targets-and-push-to`, `013-live-finder`,
   `014-goto` and `015-tracking`. `depends_on_landed` enforces this. Met: 014's US9
   landed at a43747f, and 015's US9 at a02dc35 on 2026-11-03.
2. **This trio is re-read against 015 as landed.** Done at a02dc35 (§ Refinement
   Record). The re-read covered:
   - every citation that was marked "re-check when 015 lands", each now checked against
     015's code and its marker dropped;
   - the `playwright.config.ts` layout 015's US9 left (§ Shapes that existing tests pin,
     "Playwright");
   - what 015 does with a GoTo or a direct move requested during `keyhole_wait` (nothing;
     `goto.rs` has no Track check), and that owner question 10's MCP-side refusals still
     fit (§ Decisions, "Track and the MCP moves");
   - that `live.rs` hands each live solve to the console's tracker whenever its state is
     not `off` (it does, through `Session::track_live_pointing`). Whether US6-S1's
     10-minute run fits the gates' time is not settled by reading (§ User Story 6, "The
     feed");
   - 015's owner decision on showing Stop during `keyhole_wait` (its gap-hunt finding
     13): Stop shows while Track is armed, tracking or at the keyhole, and 015's US8 is
     now the last writer of `StopButton.tsx`, `TopBar.tsx` and `App.tsx`;
   - the last writers in § Refinement Record's `shared_writes`.
3. **The owner questions are answered.** On 2026-11-03 the owner accepted every
   recommendation (spec.md § Clarifications, Session 2026-11-03), and the text matches
   them. Every story's `open_questions` is empty.
4. **The owner approvals are given**, including the `rmcp` and `base64` dependencies,
   `GET /api/mcp`, `axis_move`, `mcp.spec.ts` in the Playwright config's two lists, and
   the additive rule for 016's test module (spec § Owner approvals). Flipping to `ready`
   records them.
5. **No operator-staged files are needed.** No story adds a file over 200 KiB.

## Plan traps

Lessons from building 012 to 014 on this factory, each with its mechanism and the check
that keeps it from recurring. They bind every story. 014's plan holds the longer record
(014 plan.md § Plan traps).

1. **Gate evidence or nothing.** The judge credits only the gate results and the story's
   diff (014 trap 1). *Check:* every Then clause names a committed test, a command a gate
   runs, or the diff. FR-016's MSRV, `cargo-deny` and `cargo-machete` checks run only in
   the queue, so US1-S13 reads the manifest and the lock instead, and US7-S6 names the
   `test` gate's freshness command.
2. **A named command is read literally** (014 trap 2). The earlier T010 ran
   `cargo doc -p finder --no-deps --all-features`, which no gate runs. *Check:* every
   command a task or scenario names is copied from `ergane.yaml` verbatim (§ Verification);
   the per-file `cargo test -p finder --locked --test <file>` lines are development
   commands only.
3. **One evidence budget** (014 trap 3). *Check:* a story keeps its diff under 64 KiB,
   `Cargo.lock` included, measured before proposing with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. US1's lock diff is
   about 15 KB of that. US1 and US5 are the largest; one that cannot fit escalates rather
   than dropping a test.
4. **An additive field still breaks a whole-body test** (014 trap 4). *Check:* `motion`
   and `axis_move` are left out of the session until they have a value,
   `tests/session_api.rs` is unchanged, and neither is ever written to the state file,
   whose `StoredSession` refuses unknown fields and whose three keys
   `tests/session_cli.rs` pins.
5. **The sandbox has only the gates' tools** (014 trap 5): no `cargo-deny`, no
   `cargo-machete`, no Rust 1.96 and no `ergane`. *Check:* no task runs a tool outside the
   gates. The earlier T004 ran `cargo deny check`; it now compares the lock with
   § Dependencies.
6. **The gate's ports are the host's** (014 trap 6). *Check:* every e2e run goes through
   `npm run e2e`, which waits on one host-wide lock (#125); the Rust tests use the router
   and bind no socket.
7. **Each worktree builds on its own** (014 trap 7). *Check:* no story sets
   `CARGO_TARGET_DIR`.
8. **Fixes land on `main` first** (014 trap 8). *Check:* a story never cherry-picks or
   copies an operator's commit; it runs `git merge origin/main`.
9. **A spec fixed mid-epic needs a relaunch.** The judge grades the spec as dispatched,
   and merging a changed trio into a running story trips the dispatched-spec check.
   *Check:* a change to this trio after dispatch goes to the operator as a relaunch
   decision; no builder edits `specs/016-mcp/`.
10. **A negative assertion passes when nothing happens** (014 trap 11). Here "no axis
    moves" passes on a mount that is not connected, and a request missing `Accept` gets
    406 before any guard runs. *Check:* every such scenario shows its positive side in the
    same test (spec.md's shared setup), and the support helper sends the shared headers,
    `Host` included.
11. **`Cargo.lock` is updated by cargo, once** (014 trap 13). *Check:* US1 runs cargo
    once without `--locked`, then the one `--precise` update, and checks the lock's diff
    against § Dependencies' list; US3 runs cargo once for `base64`, whose diff is one
    line. No story runs a bare `cargo update`, and every gate stays `--locked`.
12. **`rmcp`'s config is built, not written.** `StreamableHttpServerConfig` has a private
    field, and `ServerInfo` is deprecated, so a struct literal or the old name fails under
    `-D warnings`. *Check:* the builder calls in § `rmcp` settings, compiled in a scratch
    crate.
13. **The SDK has doors of its own.** `rmcp` answers 406, 415 or 400 before any tool runs,
    including for a batch. *Check:* the door carries out stops before `rmcp` sees the
    request, and US4-S7, S8 and S11 prove it with requests `rmcp` refuses.
14. **The same function, never a copy.** A tool that re-implements a route drifts from the
    page, and a copied guard drifts from 014's. *Check:* the tools call § The shared
    functions and 014's entry points; `git grep -n 'Sun is up\|halted by\|no path keeps
    clear' crates/finder/src/mcp` finds nothing.
15. **The simulator keeps no command or heartbeat log.** Its public API reports axis state
    and the true eyepiece direction only. *Check:* the support module wraps `SimMount` in
    a recording mount, as `tests/goto_api.rs` does, and reads `LinkStatus.heartbeats` and
    `last_heartbeat_at` through `AppState::link()`. Nothing under `crates/simulator` or
    `crates/scope-node` changes.
16. **Exact equality needs one clock reading.** The session's `time`, `node`'s
    `last_frame_age_s` and the search's classification all read the console's clock.
    *Check:* exact tool-against-route comparisons run on `FixedClock` with no simulator;
    simulator comparisons leave out the fields their scenario names.
17. **A struct literal pins its fields** (014 trap 16). Since 015, `tracking.rs` builds
    a `Planner` literal too, outside every 016 story's writes. *Check:* `Planner` and
    `Progress` gain no field (§ Shapes that existing tests pin).
18. **A builder writes only its story's declared writes.** Four of 014's stories found a
    file their tasks needed but did not name. *Check:* § Refinement Record lists every file
    each story's tasks name, including the `api.rs` moves and US7's `api.ts`,
    `useSession.ts`, `App.tsx`, `StopButton.tsx` and `MountSection.tsx`; a story that
    needs another file escalates.
19. **The page copies session fields by name.** `useSession.ts` builds the page's session
    field by field, so a new field reaches no component until it is copied. *Check:*
    `McpNote.test.tsx` renders the top bar through `useSession` with a stubbed session
    that carries `motion`.
20. **Setup's tests count its fetches.** `ScopeNodeSection.test.tsx` and
    `AimTap.test.tsx` render `SetupPage` directly and pin its fetches exactly, so a
    section that fetches on mount turns them red. *Check:* `SetupPage` and `McpSection`
    fetch nothing; `App` reads `GET /api/mcp` only on `/setup` and passes it down; the
    App-level cases that reach `/setup` get a health-shaped or `{stopped: true}` answer,
    for which `McpSection` renders nothing; neither new view has a `status` or `alert`
    role (§ Shapes that existing tests pin, "The page").
21. **Playwright projects match by file name.** A new spec runs in every project whose
    match it fits, and at a02dc35 that is `default`, against a server with no simulator.
    *Check:* owner approval 15's two lists, laid out as Prettier formats them, as re-read
    at a02dc35 after 015's US9 (§ Shapes that existing tests pin, "Playwright").
22. **One `finder --simulate` server serves the whole `simulate` project** (014 trap 15).
    *Check:* `mcp.spec.ts` sets its own site, target and sync. Its `afterAll` posts
    `/api/stop`, sets frames running, deletes `/api/aim` and resets the night's model, as
    `goto.spec.ts`'s does, because `simulate.spec.ts` runs next and asserts there is no
    aim point. The project keeps 014's `workers: 1`.
23. **A spec cited before it lands drifts from its code.** Until 2026-11-03 this trio
    cited 015 from its trio. As landed, `POST /api/stop` calls `Session::stop_tracking`
    rather than `Tracker::stop` directly, and `mountMayMove`'s third argument went to
    015's `track`, so the `motion` argument planned as third is fourth. *Check:* every
    015 citation was re-read against 015's code at a02dc35 (§ Refinement Record); a
    story that finds 015's code changed since then escalates rather than re-deciding.
24. **A landed test must stay true as later stories add to the server.** `rmcp` lists
    every routed tool, so a test that pins `tools/list`'s length or exact contents breaks
    when the next story adds a tool, and that story may not edit it. *Check:* a test
    checks each named tool's presence by name; a whole-body check compares two reads of
    the same console, never a literal body that a later tool or session field would
    change.
25. **016's test module grows only by addition** (approval 17). Every 016 test binary
    includes `tests/mcp/support.rs` by `#[path]`, so a changed helper breaks an earlier
    story's test file, which the later story may not edit. *Check:* each story adds
    items to `support.rs` and changes or removes none that an earlier story put there.
26. **US5's limiter counts US4's GoTo calls.** *Check:* within one rig, `mcp_motion.rs`
    keeps `goto_target` calls at least 1 s of virtual time apart, and US5 runs
    `--test mcp_motion` among its development checks.

## Risks

| Risk | Mitigation |
|---|---|
| Anyone on the house Wi-Fi can move the telescope. | It is the owner's call. The guards bound what they can do, Stop is on every page, and the call log shows who did what. |
| An MCP client moves the telescope while someone is at the eyepiece. | No confirmation, by decision. The page says an MCP client is moving it, and Stop wins. |
| A client loses its connection mid-move. | The console owns the motion once the tool answers. `move_axis` stops at the end of its duration (at most 30 s), and the node's 1 s watchdog covers a lost console. |
| `rmcp` releases often (five in a month), and `rmcp-macros` floats. | The pin is exact, `rmcp-macros` is locked, and the endpoint's tests catch protocol changes on a deliberate update. |
| A future `rmcp` changes what it refuses before a tool runs. | Stops never depend on it: the door carries them out first. |
| Two clients at once. | One session, and the last write wins. Stop from any client ends every motion. |
| Setup lists an address the phone can't reach under WSL's NAT. | Setup says so and names the README's portproxy and mirrored-mode section. |

## Refinement Record

Reviewed on 2026-11-01 by a plate-solver refinement session against `main` at 1c928e3,
and again on 2026-11-02 against `main` at a43747f: 014's US1 to US9 landed, and 015's
trio as dispatched there (#158 included), with 015 building. The second pass applied a
read-only gap hunt of the first draft. On 2026-11-03 the owner accepted the recommended
answer to all eleven owner questions, and a third pass folded the answers in and re-read
every 015 citation against `main` at a02dc35, where 015's US1 to US9 have landed. Writes
come from the task text. Shared writes name the merge edge that orders them, and the
013, 014 and 015 stories that last wrote the same file on `main` at a02dc35. No owner
question is open; readiness waits on the owner's review and the flip.

**Criteria changed in the second pass** (reviewed with the flip):
- approvals 3 (a missing `Host` fails), 15 (`mcp.spec.ts` in two lists, as Prettier
  lays them out) and the new 17 (016's test module grows by addition);
- FR-002 (no `Host` is refused), FR-005 (`set_target` refuses other fields), FR-009 and
  FR-013 (MCP refusals while Track is on, or a direct move runs), FR-015 (the top bar's
  single Stop), SC-004 (the `support.rs` exception), and the "Peers and hosts" and "One
  motion at a time" edge cases;
- US1-S1, US2-S6, US3-S5, US4-S9 and US5-S13 (`tools/list` "includes"); US1-S7 (no
  `Host`); new US1-S16 (`set_target`'s whitelist); US2-S3 (restated in 012's relative
  form); new US4-S16 (stops from refused requests); new US5-S16 (moves refused while
  Track is on); US6-S1 (a sync away from M42, the hold timed from `tracking`) and US6-S3
  (the same second); new US6-S4 (MCP stop ends Track) and US6-S5 (Track refused during a
  direct move); US7's story text and US7-S1 (one Stop, alt/az within 1e-8°).

**Criteria changed in the third pass** (owner question 7's answer; reviewed with the
flip):
- approval 3, FR-002, FR-018 and FR-019: `/mcp` accepts the names given with 012's
  `--allow-host`, stated as that flag behaves, and MCP adds no `--mcp-allow-host`;
- approval 15: restated with the layout 015's US9 left and the one Prettier gives a fifth
  ignore entry, with the rule itself unchanged;
- US1's Independent Test, US1-S7, US1-S8, US1-S12 and US1-S14, US4-S12, US7-S2 and
  US7-S5: `--allow-host` in place of `--mcp-allow-host`.

The 015 re-read changed no scenario or FR beyond dropping its markers. Citations whose
meaning changed, all in plan.md and tasks.md: Stop's shared body now starts with
`Session::stop_tracking`, not `Tracker::stop`; Track's start is `set_track`'s call to
`Session::start_tracking`; `set_tracking` off is US4's `stop`, so it ends every motion;
`mountMayMove`'s `motion` is its fourth argument, after 015's `track`; 015 refuses Track
on for more reasons than its FR-013's four, but still not for a running direct move;
015's US8 is the last writer of `App.tsx`, `TopBar.tsx` and `StopButton.tsx`; and the
Playwright layout is the one 015's US9 left.

```yaml
version: 1
reviewed_revision: a02dc35b6014677c574c161f143c76678eae0fb4
stories:
  US1:
    tests:
      - path: crates/finder/tests/mcp_endpoint.rs
        status: new
        disposition: add
        assertion: "through the router, with Host localhost:8080 by default: initialize with no Authorization, then a tools/list with no Mcp-Session-Id that includes get_session (readOnlyHint) and set_target by name; GET is 405 with the state file byte-for-byte unchanged; with --no-mcp a POST to /mcp answers as the same POST to /no-such-path; MCP-Protocol-Version 1999-01-01 is 400 and a notification 202; a foreign Origin, a rebinding Host and no Host at all are 403 with the target unchanged; 192.168.1.20, and surface.local on a console built with --allow-host surface.local, are 200"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/mcp_endpoint.rs
        status: new
        disposition: add
        assertion: "peers 127.0.0.1, 192.168.1.30, 172.24.0.1, fe80::1, fd00::30 and ::ffff:192.168.1.30 are 200 and equal GET /api/session; 203.0.113.7 and ::ffff:203.0.113.7 are 403 for get_session and set_target with the session unchanged; a router with no connect info is 403"
        decision: "spec.md:US1-S9"
      - path: crates/finder/tests/mcp_endpoint.rs
        status: new
        disposition: add
        assertion: "on FixedClock, get_session's and set_target's structuredContent and text equal their routes' JSON exactly; GET /api/mcp lists localhost, the injected non-loopback IPv4, link-local and unique local addresses but no global IPv6 one, and surface.local under 0.0.0.0, only localhost with local_only under 127.0.0.1, and enabled false under --no-mcp"
        decision: "spec.md:US1-S11"
      - path: crates/finder/tests/mcp_endpoint.rs
        status: new
        disposition: add
        assertion: "set_target with a node token, and with a site, each answers isError with set_target changes only target, and GET /api/session is exactly unchanged"
        decision: "spec.md:US1-S16"
      - path: crates/finder/tests/api.rs
        status: existing
        disposition: preserve
        assertion: "every /api/* answer is unchanged, POST /api/stop still needs no same-origin check, and oversized uploads are still refused with 413"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/session_writes.rs
        status: existing
        disposition: preserve
        assertion: "012's same-origin and host rules for writes, including the binary's repeated --allow-host flags, pass unchanged after their checks move into check_origin and host_allowed, which the door also reads with the same allowed_hosts list"
        decision: "plan.md:The shared functions"
      - path: crates/finder/tests/session_api.rs
        status: existing
        disposition: preserve
        assertion: "whole-body session reads and PUT /api/session's answers, including its 415 for a body that is not JSON, are unchanged after session_response and edit_session are shared"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/cli.rs
        status: existing
        disposition: preserve
        assertion: "the binary's options, startup lines and --licenses output pass unchanged beside --no-mcp, with no MCP host flag added"
        decision: "spec.md:US1-S14"
    preserved_behavior:
      - "AppState::new, AppState::shared and router keep their signatures; MCP is added with AppState::with_mcp (plan.md Shapes that existing tests pin)"
      - "deny.toml, scripts/ci/ and .github/ are unchanged; cargo-deny's duplicate warnings stay warnings (plan.md Dependencies)"
      - "the lock's diff adds rmcp's 34 packages, iana-time-zone-haiku included, with rmcp-macros at 3.5.1, and changes only the entries plan.md Dependencies lists"
      - "crates/finder/tests/mcp/support.rs is created here as 016's own shared test module; later stories only add to it (spec.md Owner approvals, approval 17)"
      - "set_target's whitelist destructures SessionEdit with no .., so a later SessionEdit field fails to compile until the tool decides on it (plan.md The tool table)"
    checks:
      development:
        - "cargo test -p finder --locked --test mcp_endpoint"
        - "cargo test -p finder --locked --test session_writes --test session_api --test api"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue's ci.yml jobs: msrv on Rust 1.96, deny (advisories, bans, sources, licences), machete and coverage at 80%"
    writes:
      - Cargo.lock
      - crates/finder/Cargo.toml
      - crates/finder/src/lib.rs
      - crates/finder/src/main.rs
      - crates/finder/src/api.rs
      - crates/finder/src/mcp/mod.rs
      - crates/finder/src/mcp/door.rs
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp_endpoint.rs
      - crates/finder/tests/mcp/support.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 016-mcp
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and moves the search into api.rs"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 016-mcp
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds the session and catalog tools"
      - path: crates/finder/tests/mcp/support.rs
        other_spec: 016-mcp
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds the simulator rig"
      - path: Cargo.lock
        other_spec: 016-mcp
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US1, and adds base64 to finder's lock entry"
      - path: crates/finder/Cargo.toml
        other_spec: 016-mcp
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US1, and adds base64"
      - path: crates/finder/src/mcp/door.rs
        other_spec: 016-mcp
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US1, and carries out the stops in the door"
      - path: crates/finder/src/mcp/mod.rs
        other_spec: 016-mcp
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US1, and declares mcp::guard and mcp::limit"
      - path: Cargo.lock
        other_spec: 014-goto
        other_story: US2
        disposition: merge_order
        reason: "016 depends_on_landed 014, whose US2 is the last story to change the lockfile; main's later d1dcddb, a pulldown-cmark patch, is an operator commit"
      - path: crates/finder/Cargo.toml
        other_spec: 014-goto
        other_story: US2
        disposition: merge_order
        reason: "016 depends_on_landed 014, whose US2 last changed finder's manifest on main"
      - path: crates/finder/src/lib.rs
        other_spec: 015-tracking
        other_story: US3
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US3 last changed lib.rs (pub mod tracking)"
      - path: crates/finder/src/main.rs
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "016 depends_on_landed 013, whose US7 last changed main.rs"
      - path: crates/finder/src/api.rs
        other_spec: 015-tracking
        other_story: US7
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US7 last changed api.rs: it adds POST /api/track and has POST /api/stop call Session::stop_tracking"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/mcp_session_tools.rs
        status: new
        disposition: add
        assertion: "search_targets for orion answers {targets} equal to GET /api/targets?q=orion exactly, with the route's array as text; set_session with min_alt_deg 85 and with sun_guard false is isError with PUT /api/session's own 400 text and the session unchanged; on FixedClock with M78 at 40 N, 75 W and orion2.jpg's pointing, get_guidance is pointable, equals the session's guidance exactly, and equals pushto::turn_tilt of the two sky::alt_az positions within 1e-9"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/mcp_session_tools.rs
        status: new
        disposition: add
        assertion: "the scope node's address and token are refused with set_session's own reason and the node unchanged; with the simulated mount connected the site 40 N 75 E is refused with the site lock's reason and applied on a console with no mount; each tool equals its route under the object rule, with read-only hints, and tools/list includes all three by name"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/targets_api.rs
        status: existing
        disposition: preserve
        assertion: "GET /api/targets answers, refusals and limits are unchanged after its body moves into search"
        decision: "plan.md:The shared functions"
      - path: crates/finder/tests/session_node.rs
        status: existing
        disposition: preserve
        assertion: "PUT /api/session still sets the scope node's address and token for the page; only set_session refuses them"
        decision: "spec.md:FR-020"
    preserved_behavior:
      - "PUT /api/session's checked-edit rules and texts are unchanged; set_session reads its arguments through the same parser, and its whitelist destructures SessionEdit with no .. (plan.md The tool table)"
      - "the page can still change the site while a mount is connected; the site lock is MCP's only (spec.md Clarifications)"
      - "every item US1 put in crates/finder/tests/mcp/support.rs is unchanged, and mcp_endpoint.rs passes unchanged; US2 only adds the rig (spec.md Owner approvals, approval 17)"
    checks:
      development:
        - "cargo test -p finder --locked --test mcp_session_tools"
        - "cargo test -p finder --locked --test targets_api --test session_node"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp_session_tools.rs
      - crates/finder/tests/mcp/support.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 016-mcp
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and moves the solve into api.rs"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 016-mcp
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and adds the frame and link tools"
      - path: crates/finder/tests/mcp/support.rs
        other_spec: 016-mcp
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2, and may only add items to the support module (approval 17)"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/mcp_frames.rs
        status: new
        disposition: add
        assertion: "solve_frame of orion2.jpg as base64 gives POST /api/solve's RA, Dec and roll exactly, and get_pointing then returns the session's pointing field for field; a PGM of exactly SOLVE_BODY_LIMIT bytes is answered with a solve report of its width and height, and one a byte longer is refused with the size reason and the pointing unchanged"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/mcp_frames.rs
        status: new
        disposition: add
        assertion: "on the live loop get_pointing is within 1 arcmin of the true eyepiece at mid-exposure; 2 s after set_unresponsive(true) get_link's node is disconnected and equals the session's node except last_frame_age_s within 0.1 s; tools/list includes the three tools by name, with the read-only hints"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/api.rs
        status: existing
        disposition: preserve
        assertion: "POST /api/solve still takes the solve gate before reading its form, refuses oversized uploads with 413, and answers as before after its body moves into solve_upload"
        decision: "plan.md:The shared functions"
      - path: crates/finder/tests/link.rs
        status: existing
        disposition: preserve
        assertion: "013's link health and freshness behaviour is unchanged; get_link only reads it"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no test image is committed; the 32 MiB PGMs are synthesised at run time (spec.md Assumptions)"
      - "the lock gains only base64's line in finder's dependency list (plan.md Plan traps, trap 11)"
      - "every item US1 and US2 put in crates/finder/tests/mcp/support.rs is unchanged, and mcp_endpoint.rs and mcp_session_tools.rs pass unchanged (spec.md Owner approvals, approval 17)"
    checks:
      development:
        - "cargo test -p finder --locked --test mcp_frames"
        - "cargo test -p finder --locked --test api --test link"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue's ci.yml jobs, machete included for base64"
    writes:
      - Cargo.lock
      - crates/finder/Cargo.toml
      - crates/finder/src/api.rs
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp_frames.rs
      - crates/finder/tests/mcp/support.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 016-mcp
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and moves the starts and Stop into api.rs"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 016-mcp
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds the mount, GoTo and Stop tools"
      - path: crates/finder/tests/mcp/support.rs
        other_spec: 016-mcp
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and may only add items to the support module (approval 17)"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/mcp_motion.rs
        status: new
        disposition: add
        assertion: "describe_axes and get_axis_state equal the simulator's exactly; goto_target completes within 1 arcmin with no page action, with motion mcp/goto_target while it runs and none after; the page's POST /api/goto gives motion page/goto and POST /api/track page/track; an old state file loads with no motion, and the file never gains a motion key"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/mcp_motion.rs
        status: new
        disposition: add
        assertion: "stop_all from a second client, stop under MCP-Protocol-Version 1999-01-01 (400), a bare stop_all with no initialize, Accept or version, a foreign-Origin stop_all, and a batch with and without a foreign Origin (403 then 415) each command both axes to stop within 0.1 s, halting the GoTo with the observer stopped it and starting none; a goto_target with a rebinding Host or from the peer 203.0.113.7 gets 403 with no axis moving after an accepted call moved it; tools/list still includes stop and stop_all by name"
        decision: "spec.md:US4-S6"
      - path: crates/finder/tests/mcp_motion.rs
        status: new
        disposition: add
        assertion: "a stop_all with a rebinding Host and Origin, one with no Host, one from 203.0.113.7 and one through a router with no connect info are each answered 403, and each time both axes are commanded to stop within 0.1 s and the GoTo is halted with the observer stopped it"
        decision: "spec.md:US4-S16"
      - path: crates/finder/tests/mcp_motion.rs
        status: new
        disposition: add
        assertion: "the test token occurs in no tool result and no call-log line; goto_target then stop_all write exactly two call-log lines in the documented form"
        decision: "spec.md:US4-S14"
      - path: crates/finder/tests/goto_api.rs
        status: existing
        disposition: preserve
        assertion: "014's GoTo routes pass unchanged: refusals with their reasons, foreign writes refused, Stop, and simulate startup, after start_motion and stop are shared"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/goto_sim.rs
        status: existing
        disposition: preserve
        assertion: "014's paused-time GoTo, halts and corrections pass unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/session_cli.rs
        status: existing
        disposition: preserve
        assertion: "the state file still holds only its durable keys; motion is never written"
        decision: "spec.md:FR-014"
      - path: crates/finder/tests/goto_refusal_api.rs
        status: existing
        disposition: preserve
        assertion: "the session's goto_refusal still equals POST /api/goto's 409 text, in 014's refusal order, after the GoTo start moves into start_motion"
        decision: "plan.md:The shared functions"
      - path: crates/finder/tests/night_sim.rs
        status: existing
        disposition: preserve
        assertion: "014's night still halts with the observer stopped it after POST /api/stop's body moves into stop, with no restart for 2 s"
        decision: "plan.md:The shared functions"
      - path: crates/finder/tests/track_api.rs
        status: existing
        disposition: preserve
        assertion: "015's Track route passes unchanged after set_track's Track-on branch moves into start_motion and stop_mount's body, Session::stop_tracking first, into stop: arming behind a GoTo, the GoTo-first start, the refusal reasons, the foreign bodiless stop ending tracking with no rate after, and the readout"
        decision: "plan.md:The shared functions"
    preserved_behavior:
      - "POST /api/stop keeps its body, answer and missing same-origin check (plan.md Shapes that existing tests pin)"
      - "goto.rs, session.rs and the session file are not changed by US4"
      - "every item US1 to US3 put in crates/finder/tests/mcp/support.rs is unchanged, and their test files pass unchanged (spec.md Owner approvals, approval 17)"
      - "within one rig, mcp_motion.rs keeps goto_target calls at least 1 s of virtual time apart, so US5's limiter leaves it green (plan.md Plan traps, trap 26)"
    checks:
      development:
        - "cargo test -p finder --locked --test mcp_motion"
        - "cargo test -p finder --locked --test goto_api --test goto_sim --test session_cli --test goto_refusal_api --test night_sim --test track_api"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue's ci.yml jobs; the owner's first use with a real MCP client against the real mount, outside the factory"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/mcp/door.rs
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp_motion.rs
      - crates/finder/tests/mcp/support.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 016-mcp
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds axis_move to the session"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 016-mcp
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds the direct moves and the limiter"
      - path: crates/finder/tests/mcp/support.rs
        other_spec: 016-mcp
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and may only add items to the support module (approval 17)"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/tests/mcp_axis_moves.rs
        status: new
        disposition: add
        assertion: "with the Sun 5 deg up all three moving tools are refused with Sun is up; at dusk with a 0 deg minimum a position 13 deg from the Sun and a top-rate turn through its azimuth are refused with the planner's reason while a position 90 deg away is carried out; a position below the minimum is refused with its reason; 3 deg up under a 15 deg minimum the altitude axis climbs and stops at 20.0 +/- 0.1 s while an azimuth turn is refused; 1.2 turns is refused and 0.8 carried out; no sync is refused with the night has no sync yet"
        decision: "spec.md:US5-S1"
      - path: crates/finder/tests/mcp_axis_moves.rs
        status: new
        disposition: add
        assertion: "0.5 deg/s for 10 s stops at 10.0 +/- 0.1 s having moved 5.0 deg less the ramp within 0.1; 60 s is refused with the duration reason; a 20 s move whose client leaves stops at 20.0 +/- 0.1 s with heartbeats carrying on; sunrise stops it within 1 s with axis_move halted by the Sun guard; ten moves in 1 s give one carried out, one the mount is already moving and eight rate-limit refusals; the page's Stop halts a move with the observer stopped it; goto_target over a move is refused; tools/list includes both moves by name with descriptions that say they move a real telescope, and no result or log line holds the token"
        decision: "spec.md:US5-S8"
      - path: crates/finder/tests/mcp_axis_moves.rs
        status: new
        disposition: add
        assertion: "with Track on for M42 through POST /api/track and tracking, move_axis, goto_axis_position and goto_target are each refused with Track is on, the command log gains no go-to and no 1 deg/s rate, and track.state is still tracking 5 s later"
        decision: "spec.md:US5-S16"
      - path: crates/finder/tests/goto_refusal_api.rs
        status: existing
        disposition: preserve
        assertion: "refusal() and POST /api/goto still give the same text in the same order, Sun is up included while guidance is pointable, after prepare's shared steps move into one function"
        decision: "plan.md:Decisions and the alternatives rejected"
      - path: crates/finder/tests/night_sim.rs
        status: existing
        disposition: preserve
        assertion: "014's halts keep the observer stopped it and halted by a lost link, with no restart for 2 s, after guard() calls halt_reason"
        decision: "plan.md:Decisions and the alternatives rejected"
      - path: crates/finder/tests/second_sync_sim.rs
        status: existing
        disposition: preserve
        assertion: "its Planner literal still compiles and the second-sync planner answers as before: Planner gains methods and no field"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/keyhole_sim.rs
        status: existing
        disposition: preserve
        assertion: "015's keyhole slew, planned with tracking.rs's Planner literal and Planner::plan, passes unchanged after US5 adds Planner's methods"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/goto_api.rs
        status: existing
        disposition: preserve
        assertion: "014's refusal texts and Stop are unchanged after prepare's steps and guard()'s test are shared with the direct moves"
        decision: "plan.md:Decisions and the alternatives rejected"
      - path: crates/finder/src/goto.rs
        status: existing
        disposition: preserve
        assertion: "goto.rs's own unit tests, the cable wind and planner cases, pass unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every refusal and halt text in goto.rs stays byte-for-byte (plan.md Shapes that existing tests pin)"
      - "Progress, serialized as the session's goto, gains no field"
      - "US4's crates/finder/tests/mcp_motion.rs passes unchanged under the limiter, because its goto_target calls are at least 1 s of virtual time apart within a rig; it is not on main yet, so it is named here rather than as an existing test (plan.md Plan traps, trap 26)"
      - "every item US1 to US4 put in crates/finder/tests/mcp/support.rs is unchanged, and their test files pass unchanged (spec.md Owner approvals, approval 17)"
      - "the page's POST /api/goto and POST /api/track keep their answers; the Track is on refusal is MCP's only (plan.md Decisions, Track and the MCP moves)"
    checks:
      development:
        - "cargo test -p finder --locked --test mcp_axis_moves --test mcp_motion"
        - "cargo test -p finder --locked --test second_sync_sim --test goto_api --test goto_sim --test goto_refusal_api --test night_sim --test keyhole_sim"
        - "cargo test -p finder --locked --lib goto"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue's ci.yml jobs; the owner's first direct move from a real MCP client at the telescope, outside the factory"
    writes:
      - crates/finder/src/goto.rs
      - crates/finder/src/api.rs
      - crates/finder/src/mcp/mod.rs
      - crates/finder/src/mcp/guard.rs
      - crates/finder/src/mcp/limit.rs
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp_axis_moves.rs
      - crates/finder/tests/mcp/support.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 016-mcp
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and calls start_motion for set_tracking"
      - path: crates/finder/src/mcp/tools.rs
        other_spec: 016-mcp
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds set_tracking to the limiter"
      - path: crates/finder/tests/mcp/support.rs
        other_spec: 016-mcp
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and may only add items to the support module (approval 17)"
      - path: crates/finder/src/goto.rs
        other_spec: 014-goto
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 014, whose US8 last changed goto.rs (the shared refusal check)"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/tests/mcp_tracking.rs
        status: new
        disposition: add
        assertion: "from a sync more than 1 arcmin from M42, set_tracking on gives motion mcp/set_tracking from the call through the Track-first GoTo and while Track runs; from the moment track.state is tracking M42 holds within 1 arcmin for 10 min sampled every 1 s, fed by the rig's live loop; get_session's track readout equals GET /api/session's at the same instant; with no mount it is refused with POST /api/track's 409 reason and no token in the result or log; move_axis then two set_tracking on in 1 s refuse the third for the rate limit, and set_tracking off in the same second is not limited"
        decision: "spec.md:US6-S1"
      - path: crates/finder/tests/mcp_tracking.rs
        status: new
        disposition: add
        assertion: "stop over MCP while tracking leaves track.state off with ended_reason stopped, both axes commanded to stop within 0.1 s and no rate in the next 5 s; set_tracking on during a move_axis is refused with a direct axis move is running, Track stays off and the move still stops at the end of its duration"
        decision: "spec.md:US6-S4"
      - path: crates/finder/tests/track_api.rs
        status: existing
        disposition: preserve
        assertion: "015's Track route answers, refusals, stop and readout pass unchanged beside set_tracking, which calls the same Track start and US4's stop"
        decision: "spec.md:FR-013"
      - path: crates/finder/tests/tracking_sim.rs
        status: existing
        disposition: preserve
        assertion: "015's feed-forward tracker and its safe endings pass unchanged; tracking.rs is not edited"
        decision: "plan.md:Project Structure"
      - path: crates/finder/tests/track_cli.rs
        status: existing
        disposition: preserve
        assertion: "tracking that moved the mount is still off and quiet after a binary restart, and the imaging camera survives it; Track stays run-only"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "crates/finder/src/tracking.rs and the Track state 015 keeps in session.rs (TrackRun, Session::start_tracking, stop_tracking, track_live_pointing) are not edited"
      - "tracking is never persisted and starts off at every binary start, as 015's api.rs docs say and tests/track_cli.rs pins"
      - "every item US1 to US5 put in crates/finder/tests/mcp/support.rs is unchanged, and their test files pass unchanged (spec.md Owner approvals, approval 17)"
      - "the console's tracker is fed by 015's live.rs from the rig's solved frames, with no hook of 016's: live.rs calls Session::track_live_pointing after each live pointing, which feeds the tracker whenever its state is not off (plan.md User Story 6)"
    checks:
      development:
        - "cargo test -p finder --locked --test mcp_tracking"
        - "cargo test -p finder --locked --test track_api --test tracking_sim --test track_cli --test mcp_axis_moves"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue's ci.yml jobs; tracking from a real MCP client at the telescope, outside the factory"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/mcp/tools.rs
      - crates/finder/tests/mcp_tracking.rs
      - crates/finder/tests/mcp/support.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
  US7:
    tests:
      - path: crates/finder/web/src/components/McpNote.test.tsx
        status: new
        disposition: add
        assertion: "rendered through useSession with a stubbed session, the note shows Moved by an MCP client and the tool only while motion.by is mcp, and Stop shows while motion is present though the mount reports no movement"
        decision: "spec.md:FR-015"
      - path: crates/finder/web/src/components/McpSection.test.tsx
        status: new
        disposition: add
        assertion: "rendered directly with each status: a LAN status lists every URL in order and surface.local with the WSL NAT note; a loopback status lists only localhost with its note; an off status says MCP is off; a health-shaped answer renders nothing; no element has a status or alert role"
        decision: "spec.md:US7-S2"
      - path: crates/finder/web/e2e/mcp.spec.ts
        status: new
        disposition: add
        assertion: "in the simulate project, goto_target over HTTP shows the note in the guidance block and top bar within 1 s with the top bar's single Stop enabled; Stop halts the GoTo with the observer stopped it, and the eyepiece's alt and az change by less than 1e-8 deg over 2 s; the note and Setup's MCP section have 0 off-red pixels in night mode; afterAll stops, restarts frames, deletes the aim and resets the night's model"
        decision: "spec.md:US7-S1"
      - path: crates/finder/web/src/components/SetupPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "open() renders SetupPage with no MCP prop and so makes no new fetch; the two App cases at /setup answer GET /api/mcp with a health-shaped body, for which McpSection renders nothing, and assert on pushState, not on a fetch count"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/ScopeNodeSection.test.tsx
        status: existing
        disposition: preserve
        assertion: "its exact fetch counts on SetupPage (lines 81 and 134) hold, because SetupPage fetches nothing new"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/AimTap.test.tsx
        status: existing
        disposition: preserve
        assertion: "its exact fetch counts on SetupPage (lines 83, 113, 123 and 134) hold, because SetupPage fetches nothing new"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/MountSection.test.tsx
        status: existing
        disposition: preserve
        assertion: "the mount section's cases pass unchanged, rendered through SetupPage with no MCP prop and on its own; with no motion in the session, mountMayMove answers as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/FieldWarning.test.tsx
        status: existing
        disposition: preserve
        assertion: "the field warning's cases pass unchanged, rendered through SetupPage with no MCP prop"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/useSession.test.ts
        status: existing
        disposition: preserve
        assertion: "the hook's existing cases, including the health body's toEqual, pass unchanged because motion is passed through only when present"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/StopButton.test.tsx
        status: existing
        disposition: preserve
        assertion: "Stop's existing cases pass unchanged with two or three arguments, motion being an optional fourth after 015's track; its /setup App cases answer GET /api/mcp with {stopped: true}, for which McpSection renders nothing, and line 80 still finds one Stop"
        decision: "plan.md:User Story 7 — The page and the README"
      - path: crates/finder/web/src/components/StopTrack.test.tsx
        status: existing
        disposition: preserve
        assertion: "015's Stop-while-Track cases pass unchanged on / and /setup: Stop shows in the top bar for armed, tracking and keyhole_wait and hides for an idle mount; their stub answers GET /api/mcp with a health-shaped body, for which McpSection renders nothing"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/ImagingCameraSection.test.tsx
        status: existing
        disposition: preserve
        assertion: "015's imaging camera cases pass unchanged: setup() renders SetupPage with no MCP prop and so makes no new fetch, and the App case that opens Setup answers GET /api/mcp with a health-shaped body"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/TrackControl.test.tsx
        status: existing
        disposition: preserve
        assertion: "015's Track control cases in GuidanceBlock pass unchanged, with the MCP note placed there rendering nothing while the session has no motion"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "Setup has no off-red pixel with the MCP section on the session server, its unscoped getByRole status still finds only Site saved., and the phone's first screen is unchanged with no motion there"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/goto.spec.ts
        status: existing
        disposition: preserve
        assertion: "014's GoTo, second sync and Stop flows pass unchanged before mcp.spec.ts in the simulate project; its strict Stop locator and the guidance block's status locator each still find one element"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/simulate.spec.ts
        status: existing
        disposition: preserve
        assertion: "013's simulate flow passes unchanged after mcp.spec.ts under workers 1, starting with no aim point"
        decision: "plan.md:Plan traps"
      - path: crates/finder/web/e2e/tracking.spec.ts
        status: existing
        disposition: preserve
        assertion: "015's Track flow passes unchanged in the simulate project after goto, mcp and simulate, and its Track-region status locator still finds one element"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "package.json dependencies are unchanged, and dist is the build of its source (spec.md US7-S6)"
      - "playwright.config.ts changes only by owner approval 15: mcp.spec.ts appended to the simulate project's one-line match list and to the default project's ignore list, which Prettier then puts one entry to a line (plan.md Shapes that existing tests pin, Playwright)"
      - "every existing Vitest and Playwright test and every fixture under src/test/fixtures is unchanged"
      - "the page renders one Stop, the top bar's; the guidance block gets none (spec.md FR-015)"
    checks:
      development:
        - "cd crates/finder/web && npm test"
        - "cd crates/finder/web && CI=1 npm run e2e"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan; 8097-8099 free on the factory host while it builds (plan.md Plan traps, trap 6); the maintainer's Impeccable review after US7 lands, outside the factory"
    writes:
      - DESIGN.md
      - .impeccable/design.json
      - crates/finder/README.md
      - crates/finder/web/dist
      - crates/finder/web/playwright.config.ts
      - crates/finder/web/e2e/mcp.spec.ts
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/useSession.ts
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/components/McpNote.tsx
      - crates/finder/web/src/components/McpNote.test.tsx
      - crates/finder/web/src/components/McpSection.tsx
      - crates/finder/web/src/components/McpSection.test.tsx
      - crates/finder/web/src/components/GuidanceBlock.tsx
      - crates/finder/web/src/components/TopBar.tsx
      - crates/finder/web/src/components/StopButton.tsx
      - crates/finder/web/src/components/MountSection.tsx
      - crates/finder/web/src/components/SetupPage.tsx
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/web/playwright.config.ts
        other_spec: 015-tracking
        other_story: US9
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US9 last changed the config: it adds tracking.spec.ts to both lists and spreads the default project over four lines"
      - path: crates/finder/README.md
        other_spec: 015-tracking
        other_story: US7
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US7 last changed the README: it documents POST /api/track"
      - path: DESIGN.md
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US8 last records the Track views"
      - path: .impeccable/design.json
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US8 last records the Track views"
      - path: crates/finder/web/dist
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US8 last rebuilds the bundle"
      - path: crates/finder/web/src/api.ts
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US8 last writes api.ts (SessionTrack and the imaging camera)"
      - path: crates/finder/web/src/components/GuidanceBlock.tsx
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US8 last places a control there (TrackControl)"
      - path: crates/finder/web/src/components/SetupPage.tsx
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US8 last adds a section (ImagingCameraSection)"
      - path: crates/finder/web/src/useSession.ts
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US8 last writes useSession.ts (track, imaging_camera and backlash)"
      - path: crates/finder/web/src/App.tsx
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US8 last writes App.tsx (it passes track to TopBar)"
      - path: crates/finder/web/src/components/TopBar.tsx
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US8 last writes the top bar (it passes track to StopButton)"
      - path: crates/finder/web/src/components/StopButton.tsx
        other_spec: 015-tracking
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 015, whose US8 last writes Stop's rule (mountMayMove's third argument, track)"
      - path: crates/finder/web/src/components/MountSection.tsx
        other_spec: 014-goto
        other_story: US8
        disposition: merge_order
        reason: "016 depends_on_landed 014, whose US8 added MountSection and last changed it"
    open_questions: []
```
