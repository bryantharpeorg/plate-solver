---
state: draft
depends_on_landed: ["012-targets-and-push-to", "013-live-finder", "014-goto", "016-mcp", "017-console-stop-confirm", "019-night-legibility", "047-console-uses-the-database-that-fits", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of 2026-11-27: after a
# field night the owner picked, from the seat's options, "Console on local network +
# token". Drafted from a probe of that day on the branch `probe/056` cut from f6d4f5fd
# (four stories, tip b2513833; plan.md § What was run and what was read). Every
# `path:line` here was read at f6d4f5fd. None of the nine owner decisions is answered yet;
# the scenarios are written to each recommendation. 012 is named for the same-origin rule
# and `--allow-host`; 013 for the scope link's token comparison; 014 and 017 for Stop,
# open to anyone; 016 for MCP's door; 019 for the night edition; 017, 047 and 051 because
# each last changed a file this spec writes (plan.md § Refinement Record). 052, 053, 054
# and 055 are drafted beside this spec and land before it, in that order; it is
# re-anchored on them before any flip.
---

# Feature Specification: The Console Opens to a Paired Phone

**Feature Branch**: `056-console-opens-to-a-paired-phone`

**Created**: 2026-11-27

**Status**: Draft

**Input**: On a field night the console listened on its own machine's loopback only, so
the phone at the telescope could not open it, and the owner watched through a laptop's
ssh tunnel while a Claude session read the numbers out. Since 2026-11-21 that tunnel has
been the owner's way in ("we'll need an ssh tunnel to the finder, and we'll need
claude"), and it must keep working unchanged. After the night the owner picked, from the
seat's options, "Console on local network + token", which the seat had described as the
console also listening on the scope node's network, a phone's hotspot for example, and
asking for a pairing token, so that the phone's browser opens the page directly. Spec 055 puts nudge arrows on
the console page, which the owner wanted to press "from the phone" (the field night's
driver's words, not the owner's). Today the console has no login at all
(`crates/finder/src/main.rs:35` defaults `--listen` to 127.0.0.1:8080, and its hint
says "to open it from a phone, run with --listen 0.0.0.0:8080",
`crates/finder/src/main.rs:278`): listening on the network would hand every device on it
the page, and 016's MCP endpoint already answers any LAN peer with no token.

## Clarifications

### Session 2026-11-27 (the probe on `main` at f6d4f5fd)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. The
  probe's branch and logs are not in the repository; what they showed is written here.
- Q: How does the console serve today? → A: One axum router
  (`crates/finder/src/api.rs:616`) holds the JSON API, the live stream
  (`GET /api/live`, server-sent events, `crates/finder/src/api.rs:638`), the previews,
  the imaging routes and the finder camera's focus routes merged in
  (`crates/finder/src/api.rs:661`), `/mcp` behind its door
  (`crates/finder/src/api.rs:653`) and the embedded page as the fallback
  (`crates/finder/src/api.rs:643`). The binary binds `--listen`
  (`crates/finder/src/main.rs:258`) and serves it with the peer's address recorded
  (`crates/finder/src/main.rs:282`). No route asks who is calling. Writes carry 012's
  same-origin rule (`crates/finder/src/api.rs:537`): an `Origin`, when sent, must match,
  and its `Host` must be an IP literal, `localhost` or an `--allow-host` name
  (`crates/finder/src/api.rs:602`). MCP's door carries out any stop first
  (`crates/finder/src/mcp/door.rs:41`), then makes the Origin and Host checks
  (`crates/finder/src/mcp/door.rs:56`) and refuses a peer that is not loopback, private,
  link-local or unique-local (`crates/finder/src/mcp/door.rs:61`,
  `crates/finder/src/mcp/door.rs:78`), with no token. `POST /api/stop` answers anyone
  (014's owner approval 2).
- Q: How does the page fetch? → A: From many places: 37 `fetch` calls in `api.ts`,
  `App.tsx` and `useSession.ts`, one `EventSource` (`crates/finder/web/src/useLive.ts:22`)
  and 12 image sources, all same-origin. A header added to each would touch every one,
  and neither `EventSource` nor `<img>` can send a header. A cookie reaches all of them
  with no change to the page (run: the probe's paired phone read every route, the live
  stream and both previews with the cookie alone).
- Q: What does the scope link's pairing teach? → A: Its token is compared in constant
  time by `scope_link::tokens_match` (`crates/scope-link/src/auth.rs:13`), never logged,
  never returned by the API (`token_set` only), and kept in an owner-only file
  (`crates/finder/src/session.rs:2070`). This spec reuses all four; its stop needs no
  token, as here.
- Q: Can the factory's gates make a request from a peer that is not loopback? → A: In
  process, yes: `axum`'s `MockConnectInfo` sets the peer, as 016's tests do
  (`crates/finder/tests/support/mcp.rs:28`, `crates/finder/tests/mcp/endpoint.rs:202`).
  The gates' browsers and the built binary reach the console only over loopback, so the
  browser scenario opens the prompt at `/pair`, which answers every peer.

## Owner decisions

None is answered yet. Each keeps its options and its evidence, and the scenarios and the
record are written to the recommendations. One general rule is behind decisions 1, 4, 6
and 9: **a request from a peer that is not loopback presents the console token, as a
cookie or as a bearer header, and names an allowed host, or nothing behind it runs; only
Stop and the pairing prompt answer without it. A loopback peer is answered as today.** So
the page, its live stream, its pictures, the imaging routes, curl and MCP need no rule of
their own, no route lists who may call it, and the React page is not changed: the one
cookie the prompt sets reaches every request the page makes.

1. **Who presents the token** (FR-001 to FR-004). Recommended (a): **every peer that is
   not loopback, on every route but `POST /api/stop` and the prompt; loopback peers, the
   tunnel and a Claude session on the console's machine among them, never.** Evidence
   (run): with a token, the probe's three loopback peers (127.0.0.1, ::1,
   ::ffff:127.0.0.1) got byte-for-byte the answers of a console without one, Host or no
   Host (US1-S1); every route refused a phone without the token (US1-S2); Stop answered a
   phone with no token and a foreign Host (US1-S5). Instead: (b) **only writes**, reads
   open to the LAN as today: any device on the Wi-Fi reads the site, the session and the
   pictures; (c) **loopback peers too**: the tunnel and the Claude session would need the
   token, against the owner's 2026-11-21 path; (d) **Stop behind the token too**: against
   014's owner approval 2 ("anyone who can reach the console or the node can halt the
   telescope") and 017's FR-006.
2. **The token's form and source** (FR-012). Recommended (a): **twelve symbols of
   Crockford's base32 (60 bits) in three groups, such as `ABCD-EFGH-JKMN`, written at the
   first start to `console-token` in the state folder, owner-only, from the system's
   random source `/dev/urandom` read with the standard library; the owner may write
   their own (8 to 64 letters and digits); to change it, delete the file and start
   again.** One file is the generator, the setting and the rotation. Evidence (run):
   US3-S3 and US3-S4; with FR-005's limit a guesser gets at most 14,400 tries a day from
   one address, about 8 × 10¹³ days to try every 60-bit token. `/dev/urandom` is on all three of
   `deny.toml`'s targets (Linux on x86-64 and AArch64, macOS on AArch64) and needs no
   dependency change. Instead: (b) **`getrandom` 0.4.3**, already built into the binary
   through `rmcp` (`cargo tree -i getrandom`, run), as a direct dependency: portable, but
   a `Cargo.toml` and `Cargo.lock` change (Constitution IV, CODEOWNERS) for the same
   bytes; (c) **a flag or config value only** (`--console-token`): it shows in the process
   list and the service file; (d) **a six-digit PIN**: 10⁶ tries, about 69 days from one
   address at the limit and less from many; (e) **a new token every start**: the phone
   pairs again every night.
3. **How the owner reads it** (FR-012, FR-013). Recommended (a): **from the file, over ssh
   or the Claude session (`cat` of `console-token` in the console's state folder), as the
   README says; no route ever answers it and the console never prints it.** Evidence: no
   page change is needed, and a page on another site rebound to `localhost` through the
   tunnel can read nothing it could not read before. Instead: (b) **Setup shows it to
   loopback clients**: a new route and a page section, `dist` rebuilt; any page in the
   laptop's browser reaching the tunnel by a rebound name could read the token; (c) **a QR
   code of a pairing link**: no QR encoder is in the local crate registry or the
   page's installed packages (searched, plan.md § Constitution Check), and the constitution adds no npm
   package, so it would be written fresh; (d) **printed at start**: it would sit in the
   service's journal.
4. **How long the browser keeps it, and rotation** (FR-008, FR-009). Recommended (a):
   **an `HttpOnly`, `SameSite=Strict` cookie holding the token itself, kept 400 days
   (`Max-Age=34560000`, the most browsers keep one); a new token unpairs every browser
   at once, with no session table; a wrong cookie is cleared.** Evidence (run): US2-S2,
   US2-S3. Instead: (b) **a session cookie**: a phone's browser drops it when it closes
   tabs; (c) **a session per browser kept on the console**: unpairs one phone at a time,
   but a table to keep, expire and show; (d) **browser storage and a header on every
   request**: every `fetch` changes, and `EventSource` and `<img>` cannot send one.
5. **Plain HTTP or TLS** (FR-013). Recommended (a): **plain HTTP on a private network the
   owner trusts, such as the phone's own hotspot; the README says so.** Evidence: no new
   crate; the phone opens the page at once. Instead: (b) **TLS with a self-signed
   certificate**: a TLS stack and a certificate generator (`rustls` is in the crate
   registry but not in this workspace's `Cargo.lock`; nothing found generates
   certificates), and the phone's browser shows a full-page certificate warning to click
   through, again whenever it forgets the exception; (c) **TLS from a local authority
   installed on the phone**: a certificate to make, install and renew.
6. **`/mcp` over the network** (FR-011). Recommended (a): **the same token, as
   `Authorization: Bearer`, after the door's stops and 016's checks, which stay.**
   Evidence (run): US3-S1 and US3-S2; a stop from an unpaired device is still carried
   out. Instead: (b) **loopback only**: refuses an MCP app on the phone, and adds a rule
   of its own; (c) **as today, any LAN peer**: once the console listens on the network
   it is a way to move the telescope with no token.
7. **The default `--listen`** (FR-012). Recommended (a): **unchanged, 127.0.0.1:8080;
   the scope node's service passes --listen 0.0.0.0:8080** (plan.md § Outside the
   factory). A desk console stays on its own machine unless asked. Instead: (b)
   0.0.0.0:8080 by default, the token guarding it; (c) [::]:8080.
8. **Wrong tokens** (FR-005). Recommended (a): **per peer address, 10 wrong tokens within
   60 s of the console's clock, then 429 whatever that address presents until the oldest
   is 60 s old; at most 1,024 addresses counted, a new one refused past that.** Evidence
   (run): US1-S4, US2-S3, US3-S2. Instead: (b) **one count for every peer**: a device on
   the Wi-Fi sending wrong tokens would keep the owner's phone out; (c) **no limit**: 60 bits
   alone make guessing hopeless, but not a short token the owner writes (8 symbols).
9. **The prompt** (FR-007, FR-010). Recommended (a): **a page the console serves itself
   at every page path and at `/pair`, always DESIGN.md's night edition, with no script,
   picture or font; the React page is not changed.** A phone that opens it at night
   never flashes white. Evidence (run): US2-S4 and US2-S5. Instead: (b) **a prompt inside
   the React page**: the page's files are then served to unpaired devices, the page and
   its `dist` change, and it follows the Night switch, unknown before pairing.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes, and no `Cargo.toml` or `Cargo.lock` changes.

1. **New public API in `crates/finder`'s library** (FR-001 to FR-012): the module
   `finder::pairing` with `COOKIE`, `TOKEN_FILE`, `WRONG_TOKENS`, `WRONG_WINDOW_S`,
   `COOKIE_MAX_AGE_S`, `ConsoleToken` (`new`, `generate`, `grouped`, `load_or_create`) and
   `InvalidToken`; and `AppState::with_console_token`. No existing signature changes;
   `finder::router` keeps its signature and gains the gate.
2. **New routes** (FR-007, FR-008): `GET /pair` and `POST /pair`.
3. **A new file in the state folder** (FR-012): `console-token`, owner-only.
4. **016's FR-001 for a peer that is not loopback** (FR-011), quoted below.
5. **The binary's start** (FR-012): a start with a bad `console-token` fails; a new
   stderr line when listening off loopback.
6. **The glossary** (FR-014): **Console token** is added and **MCP client** changes.
7. **`crates/finder`'s README** (FR-013): the `--listen` row, a section and one MCP
   sentence. The pinned sentence `The imaging camera has sixteen tools of its own`
   (`crates/finder/README.md:88`) stays.
8. **A new browser test** (FR-010): `crates/finder/web/e2e/pairing.spec.ts`, in the
   default project with no change to `crates/finder/web/playwright.config.ts`.

### Other specs' requirements this spec changes

- 016's FR-001, which reads: "The console SHALL serve MCP over Streamable HTTP at `/mcp`,
  on its own address and port, needing no token". With a console token, a request from
  a peer that is not loopback needs it (FR-011); a loopback peer and a console without a
  token need none, as before.
- The glossary's **MCP client** (`GLOSSARY.md:265`), which reads: "A program that calls
  the console's MCP tools over the house LAN, such as Claude on the Surface or an app on
  the phone. It needs no token, and passes the same guards as the page, so it can move
  the telescope nowhere the page could not." FR-014 gives its new words.

No other spec's FR changes. 012's FR-014 and FR-015 (the same-origin rule) still hold
behind the gate; 012's user story 4 says "A phone on the LAN, or curl and the Rust tests,
still reach me": they do, with the token. 016's FR-002, FR-003 and FR-012 stand: the door
still carries out a stop before any check and refuses a public peer, token or not. 014's
owner approval 2 and 017's FR-004 and FR-006 stand: Stop answers everyone and stays
outside the door's checks, the token's among them.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate commands.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process; the browser scenario runs in
  `CI=1 npm run e2e`, a later command of the same gate, in `crates/finder/web`.
- **The gated console** is `crate::support::mcp::state()` (the cedar database, no
  samples) with `with_clock` of a clock the test sets, starting at T = 1,768,528,800, and
  `with_console_token(ConsoleToken::new("test-console-token"))`, served by
  `finder::router` with `MockConnectInfo` of the scenario's peer at port 40000. **The
  plain console** is `crate::support::mcp::state()` alone, served the same way.
- **The phone** is the peer 192.168.1.30, every request of its with
  `Host` 192.168.1.20:8080 unless a scenario names another; **the other phone** is
  192.168.1.31; **the loopback peers** are 127.0.0.1, ::1 and ::ffff:127.0.0.1.
  **The cookie** is `Cookie: console_token=TESTCONSOLETOKEN`; **the bearer** is
  `Authorization: Bearer test-console-token`; **the phone's origin** is
  http://192.168.1.20:8080.
- **The reads** are `GET` of `/api/session`, `/api/health`, `/api/live`,
  `/api/live/preview.jpg?seq=1`, `/api/imaging/frames/1/preview.jpg` and `/`. **The
  answer** to a request is its status, its content type and its body, or for
  `text/event-stream` its status and type alone.
- **The words.** UNPAIRED is `this device is not paired: open the console's page and
  enter the console token`; WRONG is `that is not the console token`; LIMITED is
  `too many wrong console tokens from this device: wait a minute`; BAD HOST is
  `another device must open the console at an IP address, localhost or an allowed host`.
  **The prompt with W** is an answer of content type `text/html; charset=utf-8` whose
  page holds `<h1>Pair with the finder console</h1>`, `<p role="status">W</p>` and
  `<form method="post" action="/pair">`. **A form with X** is `POST /pair` from the phone
  with `Content-Type: application/x-www-form-urlencoded`, the phone's origin and the body
  `token=X`. **The set cookie** is `console_token=TESTCONSOLETOKEN; Path=/;
  Max-Age=34560000; HttpOnly; SameSite=Strict`; **the cleared cookie** is
  `console_token=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict`.
- **The MCP rig** is the gated console served by `crate::support::mcp::app` from the
  scenario's peer, its tools called with `crate::support::mcp::call`, each request's
  `Host` set to 192.168.1.20:8080 from the phone; **the refusal with M** is a body of
  exactly `{"jsonrpc": "2.0", "error": {"code": -32001, "message": M}}`.
- **The binary** is the built `finder` with cedar's database and samples, `--state` a
  scratch folder and the scenario's `--listen`; **a start** waits at most 60 s for a
  stderr line starting `(to open it` or `other devices pair`, and **a stop** kills it.
  **The alphabet** is `0123456789ABCDEFGHJKMNPQRSTVWXYZ`.
- **The default console** is the default project's server of
  `crates/finder/web/playwright.config.ts` (`crates/finder/web/playwright.config.ts:18`,
  `crates/finder/web/playwright.config.ts:35`), the release `finder` on 127.0.0.1:8097
  with a fresh state folder, opened at 412 × 915 CSS pixels and a device scale of 2.
- **Oracles.** The answers a loopback peer and the plain console give; the status codes,
  headers and words of these scenarios; the alphabet and the file's mode; 012's, 014's,
  016's and 017's words; and DESIGN.md's night tokens, Night Ground `#000000` and the
  inks `#e00000`, `#d40000`, `#a80000` and `#5a0000`. No expected value comes from the
  code under test: no test reads `WRONG_TOKENS`, `WRONG_WINDOW_S` or `COOKIE_MAX_AGE_S`.

### User Story 1 - Another device presents the console token before any route answers (Priority: P1)

As the owner, I need the console to answer a device other than its own machine only when
that device presents the console token, and to answer its own machine, the ssh tunnel
among it, exactly as today.

**Why this priority**: The gate is the feature; the prompt, MCP, the binary's token and
the words build on it.

**Independent Test**: `crates/finder/tests/console/pairing.rs`, new, and the tests module
of `crates/finder/src/pairing.rs`. The `test` gate's nextest command runs both; during
development,
`cargo nextest run -p finder --locked --lib --test console -E 'test(/^pairing::/)'`.

**Acceptance Scenarios**:

1. **Given** the gated console and the plain console, **When** a committed test sends each of the reads from each loopback peer with `Host` localhost:8080, with `Host` 127.0.0.1:8080 and with no `Host`, and then from each `PUT /api/session` with `Host` localhost:8080, `Content-Type: application/json` and `{"target": "M42"}`, **Then** it asserts that each read's answer equals the plain console's answer to the same request from 127.0.0.1, and that each `PUT` answers 200.
2. **Given** the gated console, **When** a committed test reads `GET /api/session` from 127.0.0.1, sends from the phone with no token each of the reads, `GET /setup`, `GET /assets/index.js` and `PUT /api/session` with `Content-Type: application/json` and `{"target": "M42"}`, sends `GET /api/session` with `Host` localhost:8080 through `finder::router` with no peer recorded, and reads `GET /api/session` from 127.0.0.1 again, **Then** it asserts that every request from the phone and the one with no peer answer 401; that each of them under `/api/` answers exactly `{"error": UNPAIRED}`; and that the last read equals the first.
3. **Given** the gated console, **When** a committed test sends each of the reads from the phone four times, with the cookie, with `Cookie: theme=night; console_token=test-console-token`, with the bearer and with `Authorization: Bearer TESTCONSOLETOKEN`, then `PUT /api/session` with the cookie, the phone's origin, `Content-Type: application/json` and `{"target": "M42"}`, and reads `GET /api/session` from 127.0.0.1, **Then** it asserts that each answer equals the answer 127.0.0.1 gets to the same read with `Host` 192.168.1.20:8080 and no token, that the `PUT` answers 200, and that the session's `target.messier` is `M42`.
4. **Given** the gated console, **When** a committed test sends `GET /api/health` from the phone at T + n seconds for n from 0 to 9, with `Cookie: console_token=NOPE-NOPE`, then `Authorization: Bearer NOPE-NOPE`, then `Authorization: Basic dGVzdA==`, then seven times `Authorization: Bearer NOPE-NOPE`; at T + 59.9 s sends `GET /api/session` with the cookie and `GET /` with the bearer from the phone and `GET /api/session` with the cookie from the other phone; and at T + 60 s sends `GET /api/session` with the cookie from the phone, **Then** it asserts that the first ten answer 401 with exactly `{"error": WRONG}`; that at T + 59.9 s the phone's two answer 429, the first with exactly `{"error": LIMITED}`, and the other phone's answers 200; and that at T + 60 s the phone's answers 200.
5. **Given** the gated console with `with_allowed_hosts(["finder.example"])`, **When** a committed test sends from the phone `POST /api/stop` with `Host` rebound.example:8080 and no token, then `GET /api/session` with the cookie and `Host` rebound.example:8080, with `Host` rebound.example, with no `Host`, and with `Host` finder.example:8080, **Then** it asserts that Stop answers 200 with exactly `{"stopped": true}`; that the next three answer 403 with exactly `{"error": BAD HOST}`; and that the last answers 200.

### User Story 2 - The phone pairs once, through the console's prompt (Priority: P1)

As the owner, I need the phone's browser, opened on the console's address, to ask for
the console token once, in the night edition, and to keep it.

**Why this priority**: Without the prompt a browser has no way to present the token;
the field night's phone is the reason for the spec.

**Independent Test**: `crates/finder/tests/console/pairing_prompt.rs`, new, the tests
module of `crates/finder/src/pairing.rs`, and `crates/finder/web/e2e/pairing.spec.ts`,
new. The `test` gate runs all three; during development,
`cargo nextest run -p finder --locked --lib --test console -E 'test(/^pairing_prompt::/) | test(/^pairing::tests::/)'`
and `npx playwright test --project=default e2e/pairing.spec.ts`.

**Acceptance Scenarios**:

1. **Given** the gated console, **When** a committed test sends from the phone with no token `GET /`, `GET /setup`, `GET /assets/index.js`, `GET /pair` and `GET /api/session`, **Then** it asserts that the first three answer the prompt with `Pair this device: enter the console token.` with 401 and no `Set-Cookie`; that `GET /pair` answers the same prompt with 200; and that `GET /api/session` answers 401 with the error UNPAIRED.
2. **Given** the gated console, **When** a committed test sends a form with `test-CONSOLE+token`, then each of the reads from the phone with `Cookie: ` and the first part of that answer's `Set-Cookie` before its first `;`, then a form with `TEST%2DCONSOLE%2DTOKEN`, **Then** it asserts that the first answers 303 with `Location: /` and `Set-Cookie` exactly the set cookie; that each read's answer equals the one 127.0.0.1 gets to it with no token; and that the last form answers 303.
3. **Given** the gated console, **When** a committed test sends from the phone a form with `NOPE-NOPE`; `GET /api/session` with `Cookie: console_token=NOPE`; `GET /` with that cookie; `GET /` with `Authorization: Bearer NOPE`; six more forms with `NOPE-NOPE`; a form with `test-console-token`; `GET /` with the cookie; and from the other phone `POST /pair` with the right token and `Origin` http://elsewhere.example, **Then** it asserts that the first form answers the prompt with `That is not the console token.` with 401 and no `Set-Cookie`; that `GET /api/session` answers 401 with the error WRONG and `Set-Cookie` exactly the cleared cookie; that the next `GET /` answers that prompt with 401 and the cleared cookie, and the bearer's that prompt with 401 and no `Set-Cookie`; that the six answer 401; that the right form and the last `GET /` each answer the prompt with `Too many wrong console tokens from this device: wait a minute and try again.` with 429, the form with no `Set-Cookie`; and that the other phone's answers 403 with the error `writes require the console's own origin and an allowed host` and no `Set-Cookie`.
4. **Given** the gated console, **When** a committed test sends `GET /pair` from the phone and from 127.0.0.1 and reads each page, **Then** it asserts that each answers 200; that at least 10 `#` characters stand in each page and every one begins `#000000`, `#e00000`, `#d40000`, `#a80000` or `#5a0000`; that no page holds `rgb`, `hsl`, `white`, `<script`, `<img`, `<link` or `url(`; and that each holds `<label for="token">Console token</label>`, `<input id="token" name="token"`, `min-height: 44px` and `html, body { margin: 0; background: #000000;`.
5. **Given** the default console, **When** a committed browser test opens `/pair`, focuses the field labelled `Console token`, reads the computed `color`, `background-color`, `border-top-color`, `border-bottom-color`, `outline-color` and `caret-color` of `html`, `body` and every element in `body`, then fills `NOPE-NOPE-NOPE` and presses the button `Pair`, **Then** it asserts that `/pair` answers 200 with the heading `Pair with the finder console`; that more than 10 of those colours are not `rgba(0, 0, 0, 0)` and each of those is `rgb` or `rgba` with green and blue 0; that `html`'s background is `rgb(0, 0, 0)` and the focused field's outline `rgb(224, 0, 0)`; that the field and the button are each at least 44 px tall; and that the page then shows the status `That is not the console token.` under the same heading.

### User Story 3 - MCP from another device, and the binary's console token (Priority: P1)

As an MCP client on another device, I need the console token to reach `/mcp`, and as the
owner, I need the console to make its token at its first start and keep it, never
printing it.

**Why this priority**: Without it the binary has no token, so nothing is asked, and MCP
stays open to the network.

**Independent Test**: `crates/finder/tests/mcp/pairing.rs` and
`crates/finder/tests/cli/pairing.rs`, new, and the tests module of
`crates/finder/src/pairing.rs`. The `test` gate's nextest command runs them; during
development,
`cargo nextest run -p finder --locked --lib --test mcp --test cli -E 'test(/^pairing::/)'`.

**Acceptance Scenarios**:

1. **Given** the MCP rig, **When** a committed test reads `GET /api/session` from 127.0.0.1, calls `get_session` and `set_target` with `{"target": "M42"}` from the phone with no token and again with `Authorization: Bearer NOPE-NOPE`, reads the session again, calls `get_session` and then `set_target` with `{"target": "M42"}` from the phone with the bearer, calls `get_session` from 127.0.0.1 with no token and from 203.0.113.7 with the bearer, and calls `get_session` from the phone with no token on `crate::support::mcp::state()` with no console token, **Then** it asserts that each call with no token answers 401 with `www-authenticate` exactly `Bearer realm="finder console"` and the refusal with `MCP from another device needs the console token as a bearer token`; that each with the wrong token answers 401 with the refusal with WRONG; that the second session read equals the first; that the bearer's `get_session` answers 200 with `structuredContent` equal to that session; that its `set_target` answers 200 and the session's `target.messier` is then `M42`; that the loopback call answers 200, the call from 203.0.113.7 403, and the last 200.
2. **Given** the MCP rig with the call log, **When** a committed test calls `stop` from the phone with no token, reads the call log, calls `get_session` from the phone ten times with `Authorization: Bearer NOPE-NOPE` and once with the bearer, and sends `GET /api/session` from the phone with the cookie, **Then** it asserts that `stop` answers 401 with the refusal with `MCP from another device needs the console token as a bearer token` and that the call log is then exactly `mcp stop {} -> {"stopped":true}` and a newline; that the ten answer 401; that the bearer's call answers 429 with the refusal with LIMITED; that the `GET` answers 429; and that the call log, in lower case, holds no `test-console-token`.
3. **Given** the binary with an empty state folder, **When** a committed test starts it with --listen 127.0.0.1:0 and stops it, reads `console-token` in the state folder and its mode, starts it with --listen 0.0.0.0:0, sends `GET /api/session` with no token to 127.0.0.1 at the port it printed, and stops it, **Then** it asserts that the file is three groups of four symbols of the alphabet joined by `-`, then one newline, with mode 0600; that the first start's stderr holds the line `(to open it from a phone, run with --listen 0.0.0.0:8080)`; that the `GET` answers `HTTP/1.1 200 OK`, the second start's stderr holds the line `other devices pair with the console token in ` followed by the file's path, and the file is unchanged; and that neither stderr holds the token, with or without its dashes.
4. **Given** the binary with a state folder whose `console-token` holds `my-own-console-token` and a newline, **When** a committed test starts and stops it, then writes `tiny7` and a newline there and starts it again, waiting at most 60 s for it to exit, **Then** it asserts that the file still holds `my-own-console-token` and a newline and the first stderr does not hold it; and that the second start exits unsuccessfully within the wait, its stderr holding the file's path followed by `: a console token is 8 to 64 letters and digits; dashes and spaces are ignored`, and neither `tiny7` nor `finder console: http://`.

### User Story 4 - The README and the glossary say how a phone opens the console (Priority: P2)

As the owner, I need the README to say how a phone opens the console and where the
token is, and the glossary to name the console token.

**Why this priority**: The behaviour is whole without it; the words are how the owner
finds it on a field night.

**Independent Test**: `crates/finder/tests/console/pairing_docs.rs`, new. The `test`
gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --test console -E 'test(/^pairing_docs::/)'`.

**Acceptance Scenarios**:

1. **Given** the committed `crates/finder/README.md`, **When** a committed test reads it, **Then** it asserts that `### The console token` lies after `### From a phone` and before `## MCP clients` and holds each sentence FR-013 quotes, every run of white space read as one space; that the `--listen` row ends as FR-013 says; and that `## MCP clients` holds FR-013's MCP sentence, no `There is no token`, and still `The imaging camera has sixteen tools of its own`.
2. **Given** the committed `GLOSSARY.md`, **When** a committed test reads it, **Then** it asserts that it holds the entries **MCP client** and **Console token** whole, each with FR-014's words and `_Avoid_` line.

### Edge Cases

- **A page on another site in the phone's browser** sends the cookie only to the
  console's own host (`SameSite=Strict`), and its rebound name fails the Host check
  (FR-004): it gets 401 or 403.
- **A page on another site in the laptop's browser, through the tunnel**: a loopback
  peer, answered as today; reads were open to it before this spec and still are; writes
  still need 012's same origin. No route answers the token (owner decision 3).
- **The token changes while a phone is paired**: its next request presents a wrong
  cookie, is answered 401 with the cleared cookie, and the next page load shows the
  prompt (FR-009). A page already open shows its routes' errors until reloaded.
- **Ten wrong tries from the owner's phone**: that phone waits a minute; other devices,
  the tunnel and Stop are not affected (FR-005).
- **WSL2's NAT and Windows portproxy**: the console sees the proxy's address, not
  loopback, so a browser on Windows needs the token there; mirrored networking keeps
  Windows' own browser on loopback (README § MCP under WSL2, unchanged).
- **A request with no peer recorded** (a library server without
  `into_make_service_with_connect_info`): never loopback, so asked (FR-001).
- **`--simulate` and `--mount-sky`** keep their own state folders, so each has its own
  token (FR-012).
- **An `Authorization` header that is not a bearer** counts as a wrong token (US1-S4).

## Requirements *(mandatory)*

### Functional Requirements

**The gate** (US1):

- **FR-001**: With a console token, the console SHALL answer a request whose peer
  address is not loopback only when the request presents the token, as
  `Authorization: Bearer <token>` or as the cookie `console_token=<token>`, the header
  read first when both are sent. Two tokens SHALL be equal when they are equal after
  dropping dashes and white space and putting ASCII letters in upper case, compared with
  013's `scope_link::tokens_match` (`crates/scope-link/src/auth.rs:13`). A request that
  presents none SHALL be answered 401, under `/api/` with exactly `{"error": "this device
  is not paired: open the console's page and enter the console token"}`, and nothing
  behind it SHALL run. A request whose peer the server did not record SHALL be treated as
  not loopback (owner decision 1).
- **FR-002**: A loopback peer (IPv4 127.0.0.0/8, ::1, or IPv4-mapped IPv6 loopback)
  SHALL be answered on every route as a console without a token answers it, with or
  without a `Host`.
- **FR-003**: `POST /api/stop` SHALL answer every peer as at f6d4f5fd, whatever token
  and `Host` it sends (014's owner approval 2; 017's FR-004 and FR-006).
- **FR-004**: A request from a peer that is not loopback SHALL name in `Host` an IP
  literal, `localhost` or a name given with `--allow-host`, as 012's `host_allowed`
  reads it (`crates/finder/src/api.rs:602`), before its token is read, or be answered
  403 with exactly `{"error": "another device must open the console at an IP address,
  localhost or an allowed host"}`.
- **FR-005**: A wrong token SHALL be answered 401, under `/api/` with exactly
  `{"error": "that is not the console token"}`. A peer address that has presented 10
  wrong tokens within the last 60 s of the console's clock SHALL be answered 429, under
  `/api/` with exactly `{"error": "too many wrong console tokens from this device: wait a
  minute"}`, whatever it presents, until the oldest of those 10 is 60 s old; other
  addresses SHALL NOT be affected. At most 1,024 addresses SHALL be counted at once; past
  that a new address's wrong token is answered as one past the limit (owner decision 8).
- **FR-006**: `AppState` SHALL have no console token unless
  `AppState::with_console_token` gives it one, and without one SHALL answer every request
  as at f6d4f5fd.

**The prompt** (US2):

- **FR-007**: A request from a peer that is not loopback, for a path outside `/api/` and
  `/pair`, that presents no token, a wrong one, or comes past FR-005's limit SHALL be
  answered with the pairing prompt, a `text/html; charset=utf-8` page: 401 with
  `Pair this device: enter the console token.` or `That is not the console token.`, or
  429 with `Too many wrong console tokens from this device: wait a minute and try
  again.`. `GET /pair` SHALL answer the prompt with 200 and the first words to every peer
  (owner decision 9).
- **FR-008**: `POST /pair` SHALL be a same-origin write by 012's FR-014 check
  (`crates/finder/src/api.rs:537`) and take an `application/x-www-form-urlencoded` body
  whose `token` field, `+` and `%XX` decoded, is compared as FR-001 says and counted as
  FR-005 says. The right token SHALL be answered 303 with `Location: /` and
  `Set-Cookie: console_token=<the token, upper case, no dashes>; Path=/;
  Max-Age=34560000; HttpOnly; SameSite=Strict` (400 days, owner decision 4); a wrong one
  with the prompt, 401 and its words; a request past the limit with the prompt, 429 and
  its words; neither with a cookie.
- **FR-009**: An answer to a request whose wrong token came in the cookie SHALL carry
  `Set-Cookie: console_token=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict`.
- **FR-010**: The prompt SHALL be DESIGN.md's night edition whatever the page's Night
  switch says: its only colours Night Ground `#000000` and the inks `#e00000`, `#d40000`,
  `#a80000` and `#5a0000`; no script, image, linked stylesheet or font of its own; the
  heading `Pair with the finder console`, a status paragraph holding the words, a field
  labelled `Console token` and a button `Pair`, each control at least 44 px tall, with a
  2 px Night Ink focus outline.

**MCP and the binary** (US3):

- **FR-011**: `/mcp` from a peer that is not loopback SHALL, after its door has carried
  out any stop and made 016's Origin, Host and peer checks (016's FR-002, FR-003 and
  FR-012, which stand), ask for the console token as FR-001 and FR-005 do, counted with
  the routes' wrong tokens. No token SHALL be answered 401 with `WWW-Authenticate: Bearer
  realm="finder console"` and exactly `{"jsonrpc": "2.0", "error": {"code": -32001,
  "message": "MCP from another device needs the console token as a bearer token"}}`; a
  wrong one 401 with the message `that is not the console token`; one past the limit 429
  with the message `too many wrong console tokens from this device: wait a minute`. No
  tool SHALL run for them (owner decision 6).
- **FR-012**: `finder` SHALL read its console token from `console-token` in its state
  folder (the folder `--state` names, or within the default the `simulate/` or
  `mount-sky/` folder as at `crates/finder/src/main.rs:188` and
  `crates/finder/src/main.rs:191`) and, when there is none, write a new one there before
  it listens: 12 symbols of `0123456789ABCDEFGHJKMNPQRSTVWXYZ` (Crockford's base32, 60
  bits) from `/dev/urandom`, in three groups of four joined by `-`, then a newline,
  created owner-only (mode 0600) as the session file is
  (`crates/finder/src/session.rs:2070`) (owner decision 2). A file the owner wrote SHALL
  be kept as written. A file that is not 8 to 64 letters and digits once dashes and white
  space are dropped SHALL stop the start with an error holding the file's path followed
  by `: a console token is 8 to 64 letters and digits; dashes and spaces are ignored`,
  never the file's contents. The binary SHALL never print the token. Its loopback hint
  (`crates/finder/src/main.rs:278`) SHALL stay; on any other address it SHALL print
  `other devices pair with the console token in <path>`. `--listen` SHALL keep its
  default 127.0.0.1:8080 (owner decision 7).

**The words** (US4):

- **FR-013**: `crates/finder/README.md` SHALL, in plan.md § Interfaces' words (owner
  decisions 3 and 5):
  - end the last cell of the `--listen` row (`crates/finder/README.md:34`) with "lets
    other devices connect; they must present the console token (§ The console token). |";
  - add the section `### The console token` after the location paragraph of
    `### From a phone` (`crates/finder/README.md:61`) and before `## MCP clients`,
    holding, among the rest, "Every device other than the console's own machine must
    present the console token", "to `console-token` in its state folder, readable by its
    owner only; it never prints the token.", "The browser keeps it in a cookie for 400
    days", "sends it as `Authorization: Bearer <token>`.", "Clients on the console's own
    machine are never asked", "`POST /api/stop` and MCP's `stop` and `stop_all` stop the
    mount from any device", "A device that sends 10 wrong tokens within 60 s is answered
    429" and "The console speaks plain HTTP, so the token crosses the Wi-Fi unencrypted";
  - replace "There is no token or console confirmation for MCP calls."
    (`crates/finder/README.md:80`) with "An MCP client on the console's own machine needs
    no token; another device's client sends the console token as `Authorization: Bearer
    <token>` (§ The console token). There is no console confirmation for MCP calls."

  Sentences are compared with every run of white space as one space.
- **FR-014**: `GLOSSARY.md`'s **MCP client** SHALL read "A program that calls the
  console's MCP tools, such as Claude on the Surface or an app on the phone. On the
  console's own machine it needs no token; from another device it presents the console
  token. It passes the same guards as the page, so it can move the telescope nowhere the
  page could not.", its `_Avoid_` line unchanged, and **Console token** SHALL follow it:
  "The secret every device other than the console's own machine presents before the
  console answers it: typed once into the console's pairing prompt, which the browser
  then keeps, or sent by an MCP client as a bearer token. The console keeps it in
  `console-token` in its state folder." with `_Avoid_: password, PIN, API key, node
  token`.

### Key Entities

- **Console token**: twelve symbols (or the owner's own 8 to 64), compared without case,
  dashes or spaces; kept in `console-token`, owner-only; never printed or answered.
- **The gate**: one middleware before every route; loopback peers pass; Stop and
  `/pair` pass; `/mcp` is left to its door, which asks the same.
- **The pairing prompt**: the console's own page, red on black, at every page path and
  `/pair`.
- **The cookie**: `console_token`, holding the token, `HttpOnly`, `SameSite=Strict`,
  400 days.
- **The wrong-token count**: per peer address, a sliding 60 s on the console's clock.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `Cargo.toml`, `Cargo.lock`, `ergane.yaml` or
  `playwright.config.ts` is edited. Every existing test passes unchanged (plan.md
  § Shapes that existing tests pin). The only edits to existing test files are the `mod`
  lines of `crates/finder/tests/console/main.rs`, `crates/finder/tests/mcp/main.rs` and
  `crates/finder/tests/cli/main.rs`.
- **SC-003**: A loopback peer's answers are unchanged on every read (US1-S1); a phone
  without the token gets nothing but 401, the prompt or Stop (US1-S2, US1-S5, US2-S1,
  US3-S1); with it, every read a loopback peer gets (US1-S3, US2-S2).
- **SC-004**: The token is never printed, logged or answered (US3-S2, US3-S3, US3-S4).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd.
- **In-process peers stand for the network.** The gates make requests from a phone's
  address with `MockConnectInfo`; the built binary and the browser test reach the console
  over loopback only, and the phone on a real Wi-Fi is the field night's check (plan.md
  § Outside the factory).
- **The scope node's computer is the owner's.** Whoever can read its state folder can
  read the token; the console guards the network, not the machine.
- **Out of scope:**
  - user accounts, and a list of paired devices;
  - access from outside the local network, and TLS (owner decision 5);
  - showing the token on Setup or as a QR code (owner decision 3);
  - the scope node's service file and any firewall (a rig change, plan.md § Outside the
    factory);
  - the scope link, which keeps its own token.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-007, FR-008, FR-009, FR-010]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-011, FR-012]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-013, FR-014]
```

Four stories, each under 60 % of 65,536 B (39,321 B) at 1.35 times its probe (plan.md
§ Plan traps 1): the gate; the prompt and its cookie; MCP and the binary's token; the
words. Each merges after the one before (`depends_on_merged`): US2 adds the prompt to
US1's `pairing` module and its tests use US1's test helpers; US3 adds the token file and
MCP's check to the same module; US4's README and glossary describe what US3 makes, and
its test joins the `console` binary's `mod` lines after US2's. Plain `depends_on` would
order them without putting the earlier code in the later story's base.
