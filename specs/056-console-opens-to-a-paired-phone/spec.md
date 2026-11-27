---
state: draft
depends_on_landed: ["012-targets-and-push-to", "013-live-finder", "014-goto", "016-mcp", "017-console-stop-confirm", "019-night-legibility", "028-finder-tests-do-less", "047-console-uses-the-database-that-fits", "051-tools-page-focuses-either-camera"]
# Drafted 2026-11-27 against `main` at f6d4f5fd, at the owner's ask of 2026-11-27: after a
# field night the owner picked, from the seat's options, "Console on local network +
# token". Drafted from a probe of that day on the branch `probe/056` cut from f6d4f5fd,
# and folded the same day after cynical review round 1 (1 blocker, 6 majors, 14 minors),
# rebuilt story by story on that branch with every gate at each story's own tip
# (plan.md § What was run and what was read). Every `path:line` here was read at
# f6d4f5fd. None of the ten owner decisions is answered yet; the scenarios are written
# to each recommendation. 012 is named for the same-origin rule and `--allow-host`; 013
# for the scope link's token comparison; 014 and 017 for Stop, open to anyone; 016 for
# MCP's door; 019 for the night edition; 017, 028, 047 and 051 because each last changed
# a file this spec writes (plan.md § Refinement Record). 052, 053, 054 and 055 are drafted
# beside this spec and land before it, in that order, and 057 and 058, drafted the same
# day, write some of the same files; it is re-anchored on whichever have landed before
# any flip.
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
  with no change to the page. Read, not run: that a browser's `EventSource` and `<img>`
  send a same-origin cookie is the HTML standard's, from memory; the gates' browsers
  reach the console over loopback, where nothing is asked. Run: in process, the cookie
  the prompt sets read a live stream's first report and the preview it names, equal to
  loopback's (US2-S5).
- Q: What does the scope link's pairing teach? → A: Its token is compared in constant
  time by `scope_link::tokens_match` (`crates/scope-link/src/auth.rs:13`), never logged,
  never returned by the API (`token_set` only), and kept in an owner-only file
  (`crates/finder/src/session.rs:2070`). This spec reuses all four; its stop needs no
  token, as here.
- Q: Can the factory's gates make a request from a peer that is not loopback? → A: In
  process, yes: `axum`'s `MockConnectInfo` sets the peer, as 016's tests do
  (`crates/finder/tests/support/mcp.rs:28`, `crates/finder/tests/mcp/endpoint.rs:202`).
  The gates' browsers and the built binary reach the console only over loopback, and no
  test binds anything but loopback, so the browser scenario opens the prompt at `/pair`,
  which answers every peer of a console with a token, and the line the binary prints off
  loopback is a unit test (US3-S5).

### Session 2026-11-27 (cynical review round 1)

- Q: Where does a phone's pairing belong? → A: To the host it typed. The cookie has no
  `Domain`, so a phone paired at one IP address is asked again at another, and each
  network the node joins gives it its own; the page's Night choice
  (`localStorage` `finder.night`, `crates/finder/web/index.html:19`) is per origin too.
  The field tunnel already reaches the node by its `.local` name and `host_allowed`
  takes `--allow-host` names, so the rig passes that name (plan.md § Outside the
  factory) and the README has the owner pair once by it, at home. Whether a phone's
  browser resolves `.local` on a phone's own hotspot is read, not run: it is a step of
  the field check.
- Q: Who can stop the mount? → A: Anyone who can reach the console, and any web page
  open in a browser on that network: a cross-site `POST /api/stop` with a `text/plain`
  body needs no preflight and was answered 200 `{"stopped":true}` from the phone with
  `Origin` http://evil.example and `Host` rebound.example:8080 (review round 1). It
  stays so (owner decision 1 (f) is not recommended).
- Q: Where do the stories' shared test helpers live? → A: In `crate::support`, as the
  constitution says, as a `pub mod pairing` at the end of
  `crates/finder/tests/support/mod.rs`. A file of their own under `tests/support/`
  failed 027's layout test, which pins that folder's files
  (`crates/finder/tests/layout/main.rs:126`), at US1's tip (run).

## Owner decisions

None is answered yet. Each keeps its options and its evidence, and the scenarios and the
record are written to the recommendations. One general rule is behind decisions 1, 4, 6
and 9: **with a console token, every request but Stop names an allowed host, and one
from a peer that is not loopback also presents the token, as a cookie or as a bearer
header, or nothing behind it runs; only Stop, and the pairing prompt for the token,
answer without it. A loopback peer that names an allowed host is answered as today.**
So the page, its live stream, its pictures, the imaging routes, curl and MCP need no
rule of their own, no route lists who may call it, and the React page is not changed:
the one cookie the prompt sets reaches every request the page makes.

1. **Who presents the token, and who names an allowed host** (FR-001 to FR-004).
   Recommended (a) with (e): **every peer that is not loopback presents the token on
   every route but `POST /api/stop` and the prompt; loopback peers, the tunnel and a
   Claude session on the console's machine among them, never; and every request but
   Stop, loopback's too, names an IP literal, `localhost` or an `--allow-host` name.**
   Evidence (run): with a token, 127.0.0.1, ::1 and ::ffff:127.0.0.1 naming localhost
   or 127.0.0.1 got byte-for-byte the plain console's answers, a rebound name or no
   `Host` 403, and Stop 200 with a rebound name (US1-S1); every route refused a phone
   without the token (US1-S2); Stop answered a phone with a foreign Host (US1-S5).
   Without (e), 127.0.0.1 with `Host` rebound.example:8080 read `/api/session`, which
   holds the site, and `/api/mcp` (200, review round 1): any page in the laptop's
   browser that rebinds its name to 127.0.0.1 reads the console through the tunnel. With
   (e) every gate passed with the binary's token at US3's and US4's tips: no client of
   the console on loopback names another host (the CLI tests send an IP literal, the
   browser tests 127.0.0.1, the page's dev proxy localhost, the tunnel localhost).
   Instead: (b) **only writes**, reads open to the LAN as today: any device on the
   Wi-Fi reads the site, the session and the pictures; (c) **loopback peers present the
   token too**: the tunnel and the Claude session would need it, against the owner's
   2026-11-21 path; (d) **Stop behind the token too**: against 014's owner approval 2
   ("anyone who can reach the console or the node can halt the telescope"), 014's FR-020
   and 017's FR-006; (a) **without (e)**: reads by a rebound name through the tunnel
   stay open, as before this spec, for the same gate code (the Host check after the
   loopback pass, not before) and 403 B less of US1-S1's test (its six lines of
   rebound and missing-`Host` reads); (f) **Stop open to every device but refusing a
   foreign `Origin`**: measured, it turns the cross-site halt's 200 into 403, from the
   phone and from loopback, and keeps curl's Stop (no `Origin`) and the page's own
   (200); but it changes 014's FR-020 ("`POST /api/stop` SHALL be accepted from any
   origin"). With the check on every console four existing tests fail, 014's US6-S11
   (`crates/finder/tests/motion/goto/sim.rs:188`) and 015's foreign Stop
   (`crates/finder/tests/motion/tracking/routes.rs:209`) among them; with it only on a
   console with a token they pass, but the binary's Stop then breaks FR-020 (run). What
   it spares is a halt, which fails safe; what it risks is a refused Stop. Not
   recommended; a change to 014's owner approval 2 would be the owner's.
2. **The token's form and source** (FR-012). Recommended (a): **twelve symbols of
   Crockford's base32 (60 bits) in three groups, such as `ABCD-EFGH-JKMN`, written at the
   first start to `console-token` in the state folder, owner-only, from the system's
   random source `/dev/urandom` read with the standard library, as the simulated node's
   token already is (`crates/finder/src/simulate.rs:64`); the owner may write their own
   (8 to 64 letters and digits); to change it, delete the file and start again.** One
   file is the generator, the setting and the rotation. Evidence (run): US3-S3 and
   US3-S4. FR-005's bound is 10 wrong tokens a minute from one address, and 1,024
   addresses at once, so at most 10,240 guesses a minute for the whole console (US1-S6;
   review round 1 counted 10,240 from 1,100 addresses), 14,745,600 a day: about
   7.8 × 10¹⁰ days to try every 60-bit token, and about 95,000 days (260 years) to try
   every 8-symbol token the owner writes, 33 symbols each once O, I and L read as digits.
   `/dev/urandom` is on all three of `deny.toml`'s targets (Linux on x86-64 and AArch64,
   macOS on AArch64) and needs no dependency change. Instead: (b) **`getrandom` 0.4.3**,
   already built into the binary through `rmcp` (`cargo tree -i getrandom`, run), as a
   direct dependency: portable, but a `Cargo.toml` and `Cargo.lock` change (Constitution
   IV, CODEOWNERS) for the same bytes; (c) **a flag or config value only**
   (`--console-token`): it shows in the process list and the service file; (d) **a
   six-digit PIN**: 10⁶ tries, about 98 minutes at the console's bound, about 6.6 hours
   from the 254 addresses of an IPv4 /24; (e) **a new token every start**: the phone
   pairs again every night.
3. **How the owner reads it** (FR-012, FR-013). Recommended (a): **the owner reads
   `console-token` in the console's state folder over their own ssh, or writes their own
   token there, 8 to 64 letters and digits they can type from memory at the eyepiece;
   the README says both. No route returns the token in a body, and the console never
   prints or logs it; it travels only in the requests that present it and in the one
   `Set-Cookie` of `POST /pair`'s 303.** Evidence: no page change is needed, and no
   Claude session reads it out, since the seat never prints secrets. Instead: (b)
   **Setup shows it to loopback clients**: a new route and a page section, `dist`
   rebuilt, and safe only with decision 1 (e), else a page rebound to `localhost`
   through the tunnel could read it; (c) **a QR code of a pairing link**: no QR encoder
   is in the local crate registry or the page's installed packages (searched, plan.md
   § Constitution Check), and the constitution adds no npm package, so it would be
   written fresh, for a step done once per name; (d) **printed at start**: it would sit
   in the service's journal, which field reports copy from.
4. **How long the browser keeps it, and rotation** (FR-008, FR-009). Recommended (a):
   **an `HttpOnly`, `SameSite=Strict` cookie holding the token in its compared form,
   kept 400 days (`Max-Age=34560000`, the most browsers keep one); a new token unpairs
   every browser at once, with no session table; a wrong cookie is cleared.** Evidence
   (run): US2-S2, US2-S3, US2-S5. The cookie belongs to the host the phone typed, so a
   new address or name asks again (decision 7 gives the phone one name), and a browser
   sends it to every port of that host (RFC 6265 § 8.5, from memory), so the README says
   one console per address. `Strict` withholds it on a navigation from another site or
   app, such as a link in a message, so such a link shows a paired phone the prompt until
   it reloads; nothing the owner does opens the console that way. Instead: (b) **a
   session cookie**: a phone's browser drops it when it closes tabs; (c) **a session per
   browser kept on the console**: unpairs one phone at a time, but a table to keep,
   expire and show; (d) **browser storage and a header on every request**: every `fetch`
   changes, and `EventSource` and `<img>` cannot send one; (e) **`SameSite=Lax`**: such a
   link opens the page, and 012's Origin check still refuses a cross-site write (from
   memory of RFC 6265bis, read not run), but every top-level navigation from another
   site then carries the token.
5. **Plain HTTP or TLS** (FR-013). Recommended (a): **plain HTTP on a private network the
   owner trusts, such as the phone's own hotspot; the README says that the listener
   follows the node onto every network it joins and that the token crosses the air in
   the clear on every request, not only at pairing.** Evidence: no new crate; the phone
   opens the page at once. Instead: (b) **TLS with a self-signed certificate**: a TLS
   stack and a certificate generator (`rustls` is in the crate registry but not in this
   workspace's `Cargo.lock`; nothing found generates certificates), and the phone's
   browser shows a full-page certificate warning to click through, again whenever it
   forgets the exception; (c) **TLS from a local authority installed on the phone**: a
   certificate to make, install and renew.
6. **`/mcp` over the network** (FR-011). Recommended (a): **the same token, after the
   door's stops and 016's checks, which stay: `Authorization: Bearer`, or, as on every
   route, the cookie.** Evidence (run): US3-S1 and US3-S2; a stop from an unpaired device
   is still carried out. Instead: (b) **loopback only**: refuses an MCP app on the phone,
   and adds a rule of its own; (c) **as today, any LAN peer**: once the console listens
   on the network it is a way to move the telescope with no token; (d) **the bearer
   header only**: a rule of its own for nothing, since the cookie reaches `/mcp` only from
   a page of the console's own name (`SameSite=Strict`), which the door's Origin check
   admits anyway.
7. **The default `--listen`, and the name the phone opens** (FR-012). Recommended (a):
   **unchanged, 127.0.0.1:8080; the scope node's service passes --listen 0.0.0.0:8080
   and --allow-host with the node's `.local` name** (plan.md § Outside the factory), so
   the phone pairs once by that name on every network; off loopback the start line names
   the address another device opens, found as Setup finds it, with no traffic
   (`crates/finder/src/mcp/mod.rs:27`), and the token's file (US3-S5). A desk console
   stays on its own machine unless asked. Instead: (b) 0.0.0.0:8080 by default, the token
   guarding it; (c) [::]:8080; (d) **no `--allow-host`**: the phone pairs by IP address,
   again on each network, in the dark at the scope.
8. **Wrong tokens** (FR-005). Recommended (a): **per peer address, 10 wrong tokens within
   60 s of the console's clock, then 429 whatever that address presents until the oldest
   is 60 s old; at most 1,024 addresses counted, a new one past them answered 429 for a
   wrong token and admitted with the right one; a time later than the console's clock
   dropped, so a clock stepped back frees an address.** Evidence (run): US1-S4, US1-S6,
   US1-S7, US2-S3, US3-S2. Past the cap a new device's first wrong token is told "too
   many wrong console tokens from this device" though it sent one; waiting a minute is
   still the right advice. Instead: (b) **one count for every peer**: a device on the
   Wi-Fi sending wrong tokens would keep the owner's phone out; (c) **no limit**: 60 bits
   alone make guessing hopeless, but not a short token the owner writes (8 symbols).
9. **The prompt** (FR-007, FR-010). Recommended (a): **a page the console serves itself
   at every page path and at `/pair`, always DESIGN.md's night edition, with DESIGN.md's
   field (`DESIGN.md:272`, `DESIGN.md:273`) and night primary button (`DESIGN.md:254`),
   and no script, picture or font; the React page is not changed.** A phone that opens it
   at night never flashes white. The page on a new name follows the phone's colour scheme
   until Night is chosen there (`crates/finder/web/index.html:20`), so the README says to
   turn Night on once after pairing. The page's own fields draw their focus outline
   square, 2 px offset 2 px, and so does the prompt. Evidence (run): US2-S4 and US3-S6.
   Instead: (b) **a prompt inside the React page**: the page's files are then served to
   unpaired devices, the page and its `dist` change, and it follows the Night switch,
   unknown before pairing; (c) **the prompt turns Night on for the page**: a script on the
   prompt or a page change.
10. **How `POST /pair` reads its form** (FR-008). Recommended (a): **a hand-written
    `form_field`, `&`-split pairs with `+` and `%XX` decoded, pinned by its unit test,
    after reading at most 1,024 bytes.** Evidence (run, at US2's tip): with axum's own
    `form` feature instead, `Cargo.lock` stayed byte-for-byte the same (its
    `serde_urlencoded` 0.7.1 is already there through axum) and US2's tests passed, the
    unit test of `form_field` gone with it. The two readers gave the same answers to four
    urlencoded bodies, `test-CONSOLE+token` and `TEST%2DCONSOLE%2DTOKEN` pairing (303),
    `%zz` and a body with no `token` field getting the prompt (401); a `text/plain` body
    with the right token paired through (a) and got 415 in axum's plain text through (b).
    The story's diff did not shrink: 23,397 B with (a), 23,413 B with (b), the
    `Cargo.toml` hunk costing what the 25 fewer lines of `pairing.rs` saved. The
    browser's form sends urlencoded either way, and 012's `Origin` check still runs
    first. Instead: (b) **`axum::Form`**: one feature word in `crates/finder/Cargo.toml`,
    a change Constitution IV reads twice and CODEOWNERS holds, for a decoder of one field.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes, and no `Cargo.toml` or `Cargo.lock` changes; the shared test helpers' file
gains one module (approval 10).

1. **New public API in `crates/finder`'s library** (FR-001 to FR-012): the module
   `finder::pairing` with `COOKIE`, `TOKEN_FILE`, `WRONG_TOKENS`, `WRONG_WINDOW_S`,
   `COOKIE_MAX_AGE_S`, `ConsoleToken` (`new`, `generate`, `grouped`, `load_or_create`),
   `InvalidToken` and `listening_hint`; and `AppState::with_console_token`. No existing
   signature changes; `finder::router` keeps its signature and gains the gate.
2. **New routes** (FR-007, FR-008): `GET /pair` and `POST /pair`, on a console with a
   token only.
3. **A new file in the state folder** (FR-012): `console-token`, owner-only, written
   through `console-token.tmp`.
4. **016's FR-001 for a peer that is not loopback** (FR-011), quoted below.
5. **Every request names an allowed host** (FR-004): with a console token, every request
   but Stop and `/mcp`, a loopback peer's among them and with or without an `Origin`,
   names an IP literal, `localhost` or an `--allow-host` name, beyond 012's FR-015,
   which asks it of writes that carry an `Origin`.
6. **The binary's start** (FR-012): a start with a bad `console-token` fails; a shared
   file is kept with a warning; the line after the address line comes from
   `listening_hint`, its loopback words unchanged, new words off loopback.
7. **The glossary** (FR-014): **Console token** is added and **MCP client** changes.
8. **`crates/finder`'s README** (FR-013): the `--listen` and `--allow-host` rows, a
   section and one MCP sentence. The pinned sentence `The imaging camera has sixteen tools
   of its own` (`crates/finder/README.md:88`) stays.
9. **The main README, the safety page and the console manual** (FR-013): the passages that
   say the console has no login, that MCP or `/mcp` takes no token, that anyone on the
   network can use it, and that an unlisted name only has its writes refused
   (`README.md:134`, `README.md:256`, `docs/safety-and-security.md:58`,
   `docs/safety-and-security.md:68`, `docs/guide/console.md:55`,
   `docs/guide/console.md:96`, `docs/guide/console.md:106`, `docs/guide/console.md:162`,
   `docs/guide/console.md:169`).
10. **The stories' test rig** (SC-002): `pub mod pairing` added at the end of
    `crates/finder/tests/support/mod.rs`, so every story reaches it through
    `crate::support`; 027's layout test pins that folder's files.
11. **A new browser test** (FR-010): `crates/finder/web/e2e/pairing.spec.ts`, in the
    default project with no change to `crates/finder/web/playwright.config.ts`, written
    by US3, whose binary first has a token.

### Other specs' requirements this spec changes

- 016's FR-001, which reads: "The console SHALL serve MCP over Streamable HTTP at `/mcp`,
  on its own address and port, needing no token". With a console token, a request from
  a peer that is not loopback needs it (FR-011); a loopback peer and a console without a
  token need none, as before.
- The glossary's **MCP client** (`GLOSSARY.md:265`), a program that calls the console's
  MCP tools over the house LAN, which "needs no token, and passes the same guards as the
  page, so it can move the telescope nowhere the page could not." FR-014 gives its new
  words.

No other spec's FR changes. 012's FR-014 and FR-015 (the same-origin rule) still hold
behind the gate, and FR-004 adds a refusal for a console with a token that FR-015 does not
ask for (approval 5); 012's user story 4 says "A phone on the LAN, or curl and the Rust
tests, still reach me": they do, with the token, at an allowed host. 016's FR-002, FR-003
and FR-012 stand: the door still carries out a stop before any check and refuses a public
peer, token or not. 014's owner approval 2 and FR-020 and 017's FR-004 and FR-006 stand:
Stop answers everyone, from any origin and at any host, and stays outside the door's
checks, the token's among them.

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate commands.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process; the browser scenario runs in
  `CI=1 npm run e2e`, a later command of the same gate, in `crates/finder/web`.
- **The rig** is `crate::support::pairing`, a module at the end of
  `crates/finder/tests/support/mod.rs` that US1 writes and later stories use unchanged.
- **The gated console** is `crate::support::mcp::state()` (the cedar database, no
  samples) with `with_clock` of a clock the test sets, starting at T = 1,768,528,800, and
  `with_console_token(ConsoleToken::new("test-console-token"))`, served by
  `finder::router` with `MockConnectInfo` of the scenario's peer at port 40000. **The
  plain console** is `crate::support::mcp::state()` alone, served the same way. **The
  live console** is `crate::support::live::state(&crate::support::live::sim(), None,
  false)` (a simulated scope node, a live loop on its virtual clock) with the same
  `with_console_token`, served the same way under a paused tokio clock.
- **The phone** is the peer 192.168.1.30, every request of its with
  `Host` 192.168.1.20:8080 unless a scenario names another; **the other phone** is
  192.168.1.31; **the loopback peers** are 127.0.0.1, ::1 and ::ffff:127.0.0.1.
  **The cookie** is `Cookie: console_token=TESTCONSOLETOKEN`; **the bearer** is
  `Authorization: Bearer test-console-token`; **the phone's origin** is
  http://192.168.1.20:8080.
- **The reads** are `GET` of `/api/session`, `/api/health`, `/api/live`,
  `/api/live/preview.jpg?seq=1`, `/api/imaging/frames/1/preview.jpg` and `/`; on the
  gated and plain consoles, which have no live loop and hold no picture, `/api/live` and
  both previews answer 404, and US2-S5 reads a live stream and a held preview. **The
  answer** to a request is its status, its content type and its body.
- **The words.** UNPAIRED is `this device is not paired: open the console's page and
  enter the console token`; WRONG is `that is not the console token`; LIMITED is
  `too many wrong console tokens from this device: wait a minute`; BAD HOST is
  `open the console at an IP address, localhost or an allowed host`.
  **The prompt with W** is an answer of content type `text/html; charset=utf-8` whose
  page holds `<h1>Pair with the finder console</h1>`, `<p role="status">W</p>` and
  `<form method="post" action="/pair">`. **A form with X** is `POST /pair` from the phone
  with `Content-Type: application/x-www-form-urlencoded`, the phone's origin and the body
  `token=X`. **The set cookie** is `console_token=TESTC0NS01ET0KEN; Path=/;
  Max-Age=34560000; HttpOnly; SameSite=Strict`; **the cleared cookie** is
  `console_token=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict`.
- **The MCP rig** is the gated console served by `crate::support::mcp::app` from the
  scenario's peer, its tools called with `crate::support::mcp::call`, each request's
  `Host` set to 192.168.1.20:8080 from the phone and to localhost:8080 from 127.0.0.1
  (`crate::support::mcp::session` sends none, which FR-002 refuses once there is a
  token; run); **the refusal with M** is a body of exactly
  `{"jsonrpc": "2.0", "error": {"code": -32001, "message": M}}`.
- **The binary** is the built `finder` with cedar's database and samples, `--state` a
  scratch folder and --listen 127.0.0.1:0; **a start** waits at most 60 s for the stderr
  line `(to open it from a phone, run with --listen 0.0.0.0:8080)`, and **a stop** kills
  it. **The alphabet** is `0123456789ABCDEFGHJKMNPQRSTVWXYZ`.
- **The default console** is the default project's server of
  `crates/finder/web/playwright.config.ts` (`crates/finder/web/playwright.config.ts:18`,
  `crates/finder/web/playwright.config.ts:35`), the release `finder` on 127.0.0.1:8097
  with a fresh state folder, opened at 412 × 915 CSS pixels and a device scale of 2.
- **Oracles.** The answers a loopback peer and the plain console give; the status codes,
  headers and words of these scenarios; the alphabet and the file's mode; the live loop's
  own report and preview; 012's, 014's, 016's and 017's words; and DESIGN.md's night
  tokens, Night Ground `#000000` and the inks `#e00000`, `#d40000`, `#a80000` and
  `#5a0000`. No expected value comes from the code under test: no test reads
  `WRONG_TOKENS`, `WRONG_WINDOW_S` or `COOKIE_MAX_AGE_S`.

### User Story 1 - Another device presents the console token before any route answers (Priority: P1)

As the owner, I need the console to answer a device other than its own machine only when
that device presents the console token, and to answer its own machine, the ssh tunnel
among it, as today when it names the console's own host.

**Why this priority**: The gate is the feature; the prompt, MCP, the binary's token and
the words build on it.

**Independent Test**: `crates/finder/tests/console/pairing.rs`, new, the rig, and the
tests module of `crates/finder/src/pairing.rs`. The `test` gate's nextest command runs
them; during development,
`cargo nextest run -p finder --locked --lib --test console -E 'test(/^pairing::/)'`.

**Acceptance Scenarios**:

1. **Given** the gated console and the plain console, **When** a committed test sends each of the reads from each loopback peer with `Host` localhost:8080, with `Host` 127.0.0.1:8080, with `Host` rebound.example:8080 and with no `Host`, and then from each `POST /api/stop` with `Host` rebound.example:8080 and no body and `PUT /api/session` with `Host` localhost:8080, `Content-Type: application/json` and `{"target": "M42"}`, **Then** it asserts that each read with localhost or 127.0.0.1 answers what the plain console answers the same request from 127.0.0.1; that each read with the rebound name or no `Host` answers 403 with exactly `{"error": BAD HOST}`; that each Stop answers 200 with exactly `{"stopped": true}`; and that each `PUT` answers 200.
2. **Given** the gated console, **When** a committed test reads `GET /api/session` from 127.0.0.1, sends from the phone with no token each of the reads, `GET /setup`, `GET /assets/index.js` and `PUT /api/session` with `Content-Type: application/json` and `{"target": "M42"}`, sends `GET /api/session` with `Host` localhost:8080 through `finder::router` with no peer recorded, and reads `GET /api/session` from 127.0.0.1 again, **Then** it asserts that every request from the phone and the one with no peer answer 401; that each of them under `/api/` answers exactly `{"error": UNPAIRED}`; and that the last read equals the first.
3. **Given** the gated console, **When** a committed test sends each of the reads from the phone five times, with the cookie, with `Cookie: theme=night; console_token=test-console-token`, with the bearer, with `Authorization: bearer test-c0ns0le-t0ken` and with `Authorization: BEARER TESTCONSOLETOKEN`, then `PUT /api/session` with the cookie, the phone's origin, `Content-Type: application/json` and `{"target": "M42"}`, and reads `GET /api/session` from 127.0.0.1, **Then** it asserts that each answer equals the answer 127.0.0.1 gets to the same read with `Host` 192.168.1.20:8080 and no token, that the `PUT` answers 200, and that the session's `target.messier` is `M42`.
4. **Given** the gated console, **When** a committed test sends `GET /api/health` from the phone at T + n seconds for n from 0 to 9, with `Cookie: console_token=NOPE-NOPE`, then `Authorization: Bearer NOPE-NOPE`, then `Authorization: Basic dGVzdA==`, then seven times `Authorization: Bearer NOPE-NOPE`; at T + 59.9 s sends `GET /api/session` with the cookie and `GET /` with the bearer from the phone and `GET /api/session` with the cookie from the other phone; and at T + 60 s sends `GET /api/session` with the cookie from the phone, **Then** it asserts that the first ten answer 401 with exactly `{"error": WRONG}`; that at T + 59.9 s the phone's two answer 429, the first with exactly `{"error": LIMITED}`, and the other phone's answers 200; and that at T + 60 s the phone's answers 200.
5. **Given** the gated console with `with_allowed_hosts(["finder.example"])`, **When** a committed test sends from the phone `POST /api/stop` with `Host` rebound.example:8080 and no token, then `GET /api/session` with the cookie and `Host` rebound.example:8080, with `Host` rebound.example, with no `Host`, and with `Host` finder.example:8080, **Then** it asserts that Stop answers 200 with exactly `{"stopped": true}`; that the next three answer 403 with exactly `{"error": BAD HOST}`; and that the last answers 200.
6. **Given** the gated console, **When** a committed test sends `GET /api/health` with `Authorization: Bearer NOPE-NOPE` at T once from each of the 1,024 addresses fd00::1 to fd00::400, then from fd00::401 once with `Authorization: Bearer NOPE-NOPE` and once with the bearer, and from fd00::1 with the bearer, **Then** it asserts that each of the 1,024 answers 401 with exactly `{"error": WRONG}`; that fd00::401's wrong token answers 429 with exactly `{"error": LIMITED}` and its bearer 200; and that fd00::1's bearer answers 200.
7. **Given** the gated console, **When** a committed test sends `GET /api/health` with `Authorization: Bearer NOPE-NOPE` from the phone at T + n seconds for n from 0 to 9, then `GET /api/session` with the cookie at T + 9 s, sets the clock to T − 3,600 s and sends that request again, **Then** it asserts that the ten answer 401, the first `GET /api/session` 429, and the one after the clock stepped back 200.

### User Story 2 - The phone pairs once, through the console's prompt (Priority: P1)

As the owner, I need the phone's browser, opened on the console's address, to ask for
the console token once, in the night edition, and to keep it.

**Why this priority**: Without the prompt a browser has no way to present the token;
the field night's phone is the reason for the spec.

**Independent Test**: `crates/finder/tests/console/pairing_prompt.rs`, new, and the
tests module of `crates/finder/src/pairing.rs`. The `test` gate's nextest command runs
both; during development,
`cargo nextest run -p finder --locked --lib --test console -E 'test(/^pairing_prompt::/) | test(/^pairing::tests::/)'`.

**Acceptance Scenarios**:

1. **Given** the gated console and the plain console, **When** a committed test sends from the phone with no token to the gated console `GET /`, `GET /setup`, `GET /assets/index.js`, `GET /pair` and `GET /api/session`, and to the plain console `GET /pair`, `GET /pairx`, a form with `test-console-token` and the same form to `/pairx`, **Then** it asserts that the first three answer the prompt with `Pair this device: enter the console token.` with 401 and no `Set-Cookie`; that `GET /pair` answers the same prompt with 200; that `GET /api/session` answers 401 with the error UNPAIRED; and that on the plain console `GET /pair` answers what `GET /pairx` answers, 200, and the form what the form to `/pairx` answers, 405.
2. **Given** the gated console, **When** a committed test sends a form with `test-CONSOLE+token`, then each of the reads from the phone with `Cookie: ` and the first part of that answer's `Set-Cookie` before its first `;`, then a form with `TEST%2DCONSOLE%2DTOKEN`, then a form whose body is `token=` and 1,019 `A`s (1,025 bytes), **Then** it asserts that the first answers 303 with `Location: /` and `Set-Cookie` exactly the set cookie; that each read's answer equals the one 127.0.0.1 gets to it with no token; that the second form answers 303; and that the last answers 413 with no `Set-Cookie`.
3. **Given** the gated console, **When** a committed test sends from the phone a form with `NOPE-NOPE`; `GET /api/session` with `Cookie: console_token=NOPE`; `GET /` with that cookie; `GET /` with `Authorization: Bearer NOPE`; six more forms with `NOPE-NOPE`; a form with `test-console-token`; `GET /` with the cookie; and from the other phone `POST /pair` with the right token and `Origin` http://elsewhere.example, **Then** it asserts that the first form answers the prompt with `That is not the console token.` with 401 and no `Set-Cookie`; that `GET /api/session` answers 401 with the error WRONG and `Set-Cookie` exactly the cleared cookie; that the next `GET /` answers that prompt with 401 and the cleared cookie, and the bearer's that prompt with 401 and no `Set-Cookie`; that the six answer 401; that the right form and the last `GET /` each answer the prompt with `Too many wrong console tokens from this device: wait a minute and try again.` with 429, the form with no `Set-Cookie`; and that the other phone's answers 403 with the error `writes require the console's own origin and an allowed host` and no `Set-Cookie`.
4. **Given** the gated console, **When** a committed test sends `GET /pair` from the phone and from 127.0.0.1 and reads each page, **Then** it asserts that each answers 200; that at least 10 `#` characters stand in each page and every one begins `#000000`, `#e00000`, `#d40000`, `#a80000` or `#5a0000`; that no page holds `rgb`, `hsl`, `white`, `<script`, `<img`, `<link` or `url(`; and that each holds `<label for="token">Console token</label>`, `<input id="token" name="token"`, `min-height: 44px; border: 1px solid #e00000;`, `label { display: block; margin: 1rem 0 0.25rem; font-size: 0.875rem; font-weight: 600; color: #d40000; }`, `input { padding: 0 12px; background: transparent;`, `button { margin-top: 1rem; padding: 0 16px; background: #5a0000; font-weight: 560; }` and `html, body { margin: 0; background: #000000;`.
5. **Given** the live console, **When** a committed test subscribes to its live reports, sends a form with `test-console-token` from the phone, sends `GET /api/live` from the phone with `Cookie: ` and the first part of that answer's `Set-Cookie` before its first `;`, reads the stream's first chunk after the first report, sends `GET /api/live/preview.jpg?seq=` and that report's `preview_seq` from the phone with the same cookie and from 127.0.0.1, and sends both `GET`s from the phone with no token, **Then** it asserts that the stream answers 200 with `text/event-stream`; that its first chunk holds the line `event: live` and a `data: ` line equal to that report serialized; that the phone's preview equals 127.0.0.1's, 200 with `image/jpeg` and exactly the live loop's held JPEG of that sequence; and that both `GET`s with no token answer 401 with exactly `{"error": UNPAIRED}`.

### User Story 3 - MCP from another device, and the binary's console token (Priority: P1)

As an MCP client on another device, I need the console token to reach `/mcp`, and as the
owner, I need the console to make its token at its first start and keep it, never
printing it, and the phone's prompt to be the night edition in a real browser.

**Why this priority**: Without it the binary has no token, so nothing is asked, and MCP
stays open to the network.

**Independent Test**: `crates/finder/tests/mcp/pairing.rs`,
`crates/finder/tests/cli/pairing.rs` and `crates/finder/web/e2e/pairing.spec.ts`, new,
and the tests module of `crates/finder/src/pairing.rs`. The `test` gate runs them; during
development,
`cargo nextest run -p finder --locked --lib --test mcp --test cli -E 'test(/^pairing::/)'`
and `npx playwright test --project=default e2e/pairing.spec.ts`.

**Acceptance Scenarios**:

1. **Given** the MCP rig, **When** a committed test reads `GET /api/session` from 127.0.0.1, calls `get_session` and `set_target` with `{"target": "M42"}` from the phone with no token and again with `Authorization: Bearer NOPE-NOPE`, reads the session again, calls `get_session` from the phone with the bearer and again with the cookie, then `set_target` with `{"target": "M42"}` with the bearer, calls `get_session` from 127.0.0.1 with no token and from 203.0.113.7 with the bearer, and calls `get_session` from the phone with no token on `crate::support::mcp::state()` with no console token, **Then** it asserts that each call with no token answers 401 with `www-authenticate` exactly `Bearer realm="finder console"` and the refusal with `MCP from another device needs the console token as a bearer token`; that each with the wrong token answers 401 with the refusal with WRONG; that the second session read equals the first; that the bearer's `get_session` answers 200 with `structuredContent` equal to that session and the cookie's 200; that the `set_target` answers 200 and the session's `target.messier` is then `M42`; that the loopback call answers 200, the call from 203.0.113.7 403, and the last 200.
2. **Given** the MCP rig with the call log, **When** a committed test calls `stop` from the phone with no token, reads the call log, calls `get_session` from the phone ten times with `Authorization: Bearer NOPE-NOPE` and once with the bearer, and sends `GET /api/session` from the phone with the cookie, **Then** it asserts that `stop` answers 401 with the refusal with `MCP from another device needs the console token as a bearer token` and that the call log is then exactly `mcp stop {} -> {"stopped":true}` and a newline; that the ten answer 401; that the bearer's call answers 429 with the refusal with LIMITED; that the `GET` answers 429; and that the call log, in lower case, holds no `test-console-token`.
3. **Given** the binary with a state folder holding only `console-token.tmp` with `junk`, **When** a committed test starts and stops it, reads `console-token` in the state folder and its mode, starts it again, sends `GET /api/session` with no token and `Host` its address to the address it printed, and stops it, **Then** it asserts that the file is three groups of four symbols of the alphabet joined by `-`, then one newline, with mode 0600; that `console-token.tmp` is gone; that the first start's stderr holds the line `(to open it from a phone, run with --listen 0.0.0.0:8080)`; that the `GET` answers `HTTP/1.1 200 OK` and the file is unchanged; and that neither stderr holds the token, with or without its dashes.
4. **Given** the binary with a state folder whose `console-token` holds `my-own-console-token` and a newline at mode 0644, **When** a committed test starts and stops it, then writes `tiny7` and a newline there and starts it again, waiting at most 60 s for it to exit, **Then** it asserts that the file still holds `my-own-console-token` and a newline at mode 0644; that the first stderr holds the line of the file's path followed by `: other users can read the console token; chmod 600 the file` and, in lower case, not `my-own-console-token`; and that the second start exits unsuccessfully within the wait, its stderr holding the file's path followed by `: a console token is 8 to 64 letters and digits; dashes and spaces are ignored`, and neither `tiny7` nor `finder console: http://`.
5. **Given** `finder::pairing::listening_hint`, the file path /state/console-token and an address source giving 127.0.0.1 then 192.168.1.20, or one giving none, **When** a committed unit test asks for the line of 127.0.0.1:8080 and [::1]:8080 with the first source, 0.0.0.0:8080 and [::]:8081 with the first, 192.168.1.21:8080 with none and 0.0.0.0:8080 with none, **Then** it asserts that the first two are `(to open it from a phone, run with --listen 0.0.0.0:8080)`; that the next three are `other devices open http://` followed by 192.168.1.20:8080, 192.168.1.20:8081 and 192.168.1.21:8080, each followed by ` and pair with the console token in /state/console-token`; and that the last is `other devices pair with the console token in /state/console-token`.
6. **Given** the default console, **When** a committed browser test opens `/pair`, focuses the field labelled `Console token`, reads the computed `color`, `background-color`, `border-top-color`, `border-bottom-color`, `outline-color` and `caret-color` of `html`, `body` and every element in `body`, then fills `NOPE-NOPE-NOPE` and presses the button `Pair`, **Then** it asserts that `/pair` answers 200 with the heading `Pair with the finder console`; that more than 10 of those colours are not `rgba(0, 0, 0, 0)` and each of those is `rgb` or `rgba` with green and blue 0; that `html`'s background is `rgb(0, 0, 0)` and the focused field's outline `rgb(224, 0, 0)`; that the field and the button are each at least 44 px tall; and that the page then shows the status `That is not the console token.` under the same heading.

### User Story 4 - The READMEs, the manuals and the glossary say how a phone opens the console (Priority: P2)

As the owner, I need the READMEs and the manuals to say how a phone opens the console,
where the token is and what it does not guard, and the glossary to name the console
token.

**Why this priority**: The behaviour is whole without it; the words are how the owner
finds it on a field night, and the pages that say the console has no login would be
false.

**Independent Test**: `crates/finder/tests/console/pairing_docs.rs`, new. The `test`
gate's nextest command runs it; during development,
`cargo nextest run -p finder --locked --test console -E 'test(/^pairing_docs::/)'`.

**Acceptance Scenarios**:

1. **Given** the committed `crates/finder/README.md`, **When** a committed test reads it with every run of white space as one space, **Then** it asserts that `### The console token` lies after `### From a phone` and before `## MCP clients` and holds each sentence FR-013 quotes for it; that the README holds FR-013's `--listen` and `--allow-host` rows and its MCP sentences, and still `The imaging camera has sixteen tools of its own`; and that it holds no `There is no token`.
2. **Given** the committed `GLOSSARY.md`, **When** a committed test reads it, **Then** it asserts that it holds the entries **MCP client** and **Console token** whole and in that order, each with FR-014's words and `_Avoid_` line.
3. **Given** the committed `README.md`, `docs/safety-and-security.md` and `docs/guide/console.md`, **When** a committed test reads each with every run of white space as one space, **Then** it asserts that each holds the sentences FR-013 gives it, and none of `MCP calls carry no token.`, `anyone on your network can move`, `The console has no login.`, `` `/mcp` has no token. ``, `anyone who can reach that port`, `Anyone on that network can then use`, `Otherwise writes are refused.` and `stateless and takes no token.`.

### Edge Cases

- **A page on another site in the phone's browser** sends the cookie only to the
  console's own host (`SameSite=Strict`), and its rebound name fails the Host check
  (FR-004): it gets 401 or 403 on every route but Stop. Stop it can send, with no
  preflight, from any page in any browser on the network, before this spec and after
  (FR-003, owner decision 1 (f)).
- **A page on another site in the laptop's browser, through the tunnel**: a loopback
  peer, but its rebound name fails the Host check (FR-004), so it reads nothing; before
  this spec it could read. No route returns the token in a body (owner decision 3).
- **The token changes while a phone is paired**: its next request presents a wrong
  cookie, is answered 401 with the cleared cookie, and the next page load shows the
  prompt (FR-009). A page already open shows its routes' errors until reloaded.
- **The phone opens the console at another address or name**: its cookie belongs to the
  host it typed, so it is asked again there, and the page's Night choice is that
  origin's (owner decisions 4 and 7).
- **Another HTTP service at the console's address**: the browser sends it the cookie
  too, whatever its port, and two consoles at one address overwrite each other's cookie;
  the README says one console per address.
- **Ten wrong tries from the owner's phone**: that phone waits a minute; other devices,
  the tunnel and Stop are not affected (FR-005).
- **The console's clock steps back**: wrong tokens counted after the new time are
  dropped, and the phone is not kept waiting (FR-005).
- **WSL2's NAT and Windows portproxy**: the console sees the proxy's address, not
  loopback, so a browser on Windows needs the token there; mirrored networking keeps
  Windows' own browser on loopback (README § MCP under WSL2, unchanged).
- **A request with no peer recorded** (a library server without
  `into_make_service_with_connect_info`): never loopback, so asked (FR-001).
- **A loopback client that sends no `Host`** (an HTTP/1.0 tool): refused 403 with BAD
  HOST (FR-004); every client of the console the repository holds sends one.
- **`--simulate` and `--mount-sky`** keep their own state folders, so each has its own
  token (FR-012).
- **An `Authorization` header that is not a bearer** counts as a wrong token (US1-S4).
- **The node's other ports.** The scope link's 50051 and INDI's 7624 have their own
  doors, or none (`docs/safety-and-security.md:73`, `docs/safety-and-security.md:86`);
  the rig keeps them off the network (plan.md § Outside the factory).

## Requirements *(mandatory)*

### Functional Requirements

**The gate** (US1):

- **FR-001**: With a console token, the console SHALL answer a request whose peer
  address is not loopback only when the request presents the token, as
  `Authorization: Bearer <token>`, the scheme in any case, or as the cookie
  `console_token=<token>`, the header read first when both are sent; an
  `Authorization` header of another scheme presents a wrong token. Two tokens SHALL be
  equal when they are equal after dropping dashes and white space, putting ASCII letters
  in upper case and reading O as 0 and I and L as 1 (Crockford's base32), compared with
  013's `scope_link::tokens_match` (`crates/scope-link/src/auth.rs:13`). A request that
  presents none SHALL be answered 401, under `/api/` with exactly `{"error": "this device
  is not paired: open the console's page and enter the console token"}`, and nothing
  behind it SHALL run. A request whose peer the server did not record SHALL be treated as
  not loopback (owner decision 1).
- **FR-002**: A loopback peer (IPv4 127.0.0.0/8, ::1, or IPv4-mapped IPv6 loopback) that
  names an allowed host (FR-004) SHALL never be asked for the token and SHALL be answered
  on every route but `/pair` as a console without a token answers it; its `POST /pair`
  is counted as FR-005 says, under its own address.
- **FR-003**: `POST /api/stop` SHALL answer every peer as at f6d4f5fd, whatever token,
  `Host` and `Origin` it sends (014's owner approval 2 and FR-020; 017's FR-004 and
  FR-006).
- **FR-004**: With a console token, every request but `POST /api/stop` and `/mcp`, a
  loopback peer's among them, SHALL name in `Host` an IP literal, `localhost` or a name
  given with `--allow-host`, as 012's `host_allowed` reads it
  (`crates/finder/src/api.rs:602`), before its token is read, or be answered 403 with
  exactly `{"error": "open the console at an IP address, localhost or an allowed host"}`;
  a request with no `Host` SHALL be answered so too. `/mcp` keeps its door's own check
  (016's FR-002) (owner decision 1 (e)).
- **FR-005**: A wrong token SHALL be answered 401, under `/api/` with exactly
  `{"error": "that is not the console token"}`. A peer address that has presented 10
  wrong tokens within the last 60 s of the console's clock SHALL be answered 429, under
  `/api/` with exactly `{"error": "too many wrong console tokens from this device: wait a
  minute"}`, whatever it presents, until the oldest of those 10 is 60 s old; other
  addresses SHALL NOT be affected. A wrong token counted at a time later than the
  console's clock SHALL be dropped. At most 1,024 addresses SHALL be counted at once;
  past that a new address's wrong token is answered as one past the limit, and its right
  token is admitted (owner decision 8).
- **FR-006**: `AppState` SHALL have no console token unless
  `AppState::with_console_token` gives it one, and without one SHALL answer every request
  as at f6d4f5fd, `/pair` among them.

**The prompt** (US2):

- **FR-007**: A request from a peer that is not loopback, for a path outside `/api/` and
  `/pair`, that presents no token, a wrong one, or comes past FR-005's limit SHALL be
  answered with the pairing prompt, a `text/html; charset=utf-8` page: 401 with
  `Pair this device: enter the console token.` or `That is not the console token.`, or
  429 with `Too many wrong console tokens from this device: wait a minute and try
  again.`. On a console with a token, `GET /pair` SHALL answer the prompt with 200 and
  the first words to every peer that names an allowed host (owner decision 9).
- **FR-008**: `POST /pair` SHALL be a same-origin write by 012's FR-014 check
  (`crates/finder/src/api.rs:537`) and read its body, whatever its `Content-Type`, as
  `application/x-www-form-urlencoded`, at most 1,024 bytes, a larger one answered 413 with
  no cookie; its `token` field, `+` and `%XX` decoded, is compared as FR-001 says and
  counted as FR-005 says (owner decision 10). The right token SHALL be answered 303 with
  `Location: /` and `Set-Cookie: console_token=<the token in its compared form>; Path=/;
  Max-Age=34560000; HttpOnly; SameSite=Strict` (400 days, owner decision 4); a wrong one
  with the prompt, 401 and its words; a request past the limit with the prompt, 429 and
  its words; neither with a cookie.
- **FR-009**: An answer to a request whose wrong token came in the cookie SHALL carry
  `Set-Cookie: console_token=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict`.
- **FR-010**: The prompt SHALL be DESIGN.md's night edition whatever the page's Night
  switch says: its only colours Night Ground `#000000` and the inks `#e00000`, `#d40000`,
  `#a80000` and `#5a0000`; no script, image, linked stylesheet or font of its own; the
  heading `Pair with the finder console`, a status paragraph holding the words, a field
  labelled `Console token` and a button `Pair`. The field SHALL be DESIGN.md's
  (`DESIGN.md:272`): a 1 px Night Ink box with a transparent ground and 12 px side
  padding, its label 0.875rem at weight 600 in `#d40000`; the button its night primary
  (`DESIGN.md:254`): a `#5a0000` fill with Night Ink text and border, 16 px side padding,
  weight 560; each control at least 44 px tall, with a 2 px Night Ink focus outline
  offset 2 px (`DESIGN.md:273`).

**MCP and the binary** (US3):

- **FR-011**: `/mcp` from a peer that is not loopback SHALL, after its door has carried
  out any stop and made 016's Origin, Host and peer checks (016's FR-002, FR-003 and
  FR-012, which stand), ask for the console token as FR-001 and FR-005 do, as a bearer
  header or the cookie, counted with the routes' wrong tokens. No token SHALL be answered
  401 with `WWW-Authenticate: Bearer realm="finder console"` and exactly
  `{"jsonrpc": "2.0", "error": {"code": -32001, "message": "MCP from another device
  needs the console token as a bearer token"}}`; a wrong one 401 with the message `that
  is not the console token`; one past the limit 429 with the message `too many wrong
  console tokens from this device: wait a minute`. No tool SHALL run for them (owner
  decision 6).
- **FR-012**: `finder` SHALL read its console token from `console-token` in its state
  folder (the folder `--state` names, or within the default the `simulate/` or
  `mount-sky/` folder as at `crates/finder/src/main.rs:188` and
  `crates/finder/src/main.rs:191`) and, when there is none, write a new one there before
  it listens: 12 symbols of `0123456789ABCDEFGHJKMNPQRSTVWXYZ` (Crockford's base32, 60
  bits) from `/dev/urandom`, in three groups of four joined by `-`, then a newline,
  written owner-only (mode 0600) to `console-token.tmp`, any left by an interrupted start
  removed first, and renamed, as the session file is (`crates/finder/src/session.rs:2058`)
  (owner decision 2). A file the owner wrote SHALL be kept as written; one its group or
  other users can read SHALL also be warned of on stderr with its path followed by `:
  other users can read the console token; chmod 600 the file`. A file that is not 8 to 64
  letters and digits once dashes and white space are dropped SHALL stop the start with an
  error holding the file's path followed by `: a console token is 8 to 64 letters and
  digits; dashes and spaces are ignored`, never the file's contents. The binary SHALL
  never print the token. After its address line it SHALL print
  `finder::pairing::listening_hint`'s line: on a loopback address the hint of
  `crates/finder/src/main.rs:278`, unchanged; elsewhere `other devices open
  http://<address>:<port> and pair with the console token in <path>`, the address the
  bound one or, for an unspecified one, the first address that is not loopback of the
  outward-address source Setup uses (`crates/finder/src/mcp/mod.rs:27`, which sends
  nothing), or with none `other devices pair with the console token in <path>`.
  `--listen` SHALL keep its default 127.0.0.1:8080 (owner decision 7).

**The words** (US4):

- **FR-013**: The pages SHALL say, in plan.md § Interfaces' words (owner decisions 3, 4,
  5 and 7):
  - in `crates/finder/README.md`: the `--listen` row (`crates/finder/README.md:34`)
    ending "lets other devices connect; they must present the console token (§ The
    console token). |"; the `--allow-host` row (`crates/finder/README.md:43`) as "DNS
    name the console answers, in addition to IP literals and `localhost`; any other name
    gets only `POST /api/stop`. Repeat for multiple names."; the section
    `### The console token` after the location paragraph of `### From a phone`
    (`crates/finder/README.md:61`) and before `## MCP clients`, holding, among the rest,
    "Every device other than the console's own machine must present the console token
    before the console answers it.", "to `console-token` in its state folder, readable by
    its owner only; it never prints the token.", "Read it over your own ssh,", "Or write
    your own token there, 8 to 64 letters and digits you can type from memory at the
    eyepiece,", "Pair the phone once, at home:", "The browser keeps it in a cookie for 400
    days, for that name only, so a new address or name asks again.", "If the phone cannot
    find the name, open the address the console's start line prints instead.", "Then turn
    Night on once; the page keeps it for the same name.", "sends the token as
    `Authorization: Bearer <token>`.", "Clients on the console's own machine are never
    asked for the token,", "Every request but `POST /api/stop` must open the console at an
    IP address,", "`POST /api/stop` and MCP's `stop` and `stop_all` stop the mount from any
    device and any web page, paired or not.", "A device that sends 10 wrong tokens within
    60 s is answered 429", "so run one console per address." and "The console listens on
    every network its computer joins, and it speaks plain HTTP, so the token crosses the
    air unencrypted on every request."; and "There is no token or console confirmation for
    MCP calls." (`crates/finder/README.md:80`) replaced by "An MCP client on the console's
    own machine needs no token; another device's client sends the console token as
    `Authorization: Bearer <token>` (§ The console token). There is no console
    confirmation for MCP calls.";
  - in `README.md`, at `README.md:134` and `README.md:256`: "and MCP calls from its own
    machine carry no token.", "every other device must present the console token, a phone
    once through the console's prompt and an MCP client as a bearer token, though any
    device can still stop the mount." and "The
    console asks other devices for a token but speaks plain HTTP,";
  - in `docs/safety-and-security.md`'s Network exposure (`docs/safety-and-security.md:58`
    to `docs/safety-and-security.md:69`): "**The console asks every other device for a
    token, over plain HTTP.**", "it listens on every network its computer joins.", "**The
    token crosses the air in the clear** on every request,", "**Any device, and any web
    page open in a browser on that network, can stop the mount.**", "**Every request but
    Stop must name an allowed host,** from the console's own computer too," and
    "**`/mcp` takes the same token from another device.**";
  - in `docs/guide/console.md`, at `docs/guide/console.md:55`, `:96`, `:106`, `:162` and
    `:169`: "besides IP addresses and `localhost`; any other name gets only Stop.", "**The
    phone asks for the console token once.**", "since the browser keeps the pairing for
    that name only.", "Every other device must present the console token, but the console
    speaks plain HTTP and any device can stop the mount.", "Otherwise the console answers
    that name only with Stop.", "stateless, and on the console's own machine it takes no
    token." and "and the console token as `Authorization: Bearer <token>`.".

  Sentences are compared with every run of white space as one space.
- **FR-014**: `GLOSSARY.md`'s **MCP client** SHALL read "A program that calls the
  console's MCP tools, such as a Claude session on the owner's laptop or an app on the
  phone. On the console's own machine it needs no token; from another device it presents
  the console token. It passes the same guards as the page, so it can move the telescope
  nowhere the page could not.", its `_Avoid_` line unchanged, and **Console token** SHALL
  follow it: "The secret every device other than the console's own machine presents
  before the console answers it: typed once into the console's pairing prompt, which the
  browser then keeps in a cookie, or sent by an MCP client as a bearer token. The console
  keeps it in `console-token` in its state folder. Typing it into a browser pairs that
  device with the console, which is not the scope link's pairing of the console with a
  scope node." with `_Avoid_: password, PIN, API key, node token`.

### Key Entities

- **Console token**: twelve symbols (or the owner's own 8 to 64), compared without case,
  dashes or spaces, O read as 0 and I and L as 1; kept in `console-token`, owner-only;
  never printed, logged or returned in a body.
- **The gate**: one middleware before every route; Stop and `/mcp` pass; every other
  request names an allowed host; loopback peers then pass, and `/pair` too; `/mcp` is
  left to its door, which asks the same.
- **The pairing prompt**: the console's own page, red on black, at every page path and
  `/pair`.
- **The cookie**: `console_token`, holding the token, `HttpOnly`, `SameSite=Strict`,
  400 days, for the host the browser typed.
- **The wrong-token count**: per peer address, a sliding 60 s on the console's clock, at
  most 1,024 addresses.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass at each story's tip.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `Cargo.toml`, `Cargo.lock`, `ergane.yaml` or
  `playwright.config.ts` is edited. Every existing test passes unchanged (plan.md
  § Shapes that existing tests pin). The only edits to existing test files are the `mod`
  lines of `crates/finder/tests/console/main.rs`, `crates/finder/tests/mcp/main.rs` and
  `crates/finder/tests/cli/main.rs`, and the rig's module at the end of
  `crates/finder/tests/support/mod.rs` (owner approval 10).
- **SC-003**: A loopback peer's answers are unchanged on every read at the console's own
  host (US1-S1); a phone without the token gets nothing but 401, 403, the prompt or Stop
  (US1-S2, US1-S5, US2-S1, US3-S1); with it, every read a loopback peer gets, the live
  stream and a held preview among them (US1-S3, US2-S2, US2-S5).
- **SC-004**: The token is never printed or logged, and no answer's body holds it
  (US3-S2, US3-S3, US3-S4); it travels only in the requests that present it and in
  `POST /pair`'s `Set-Cookie`.

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed. Every
  citation here was read at f6d4f5fd.
- **In-process peers stand for the network.** The gates make requests from a phone's
  address with `MockConnectInfo`; the built binary and the browser test reach the console
  over loopback only, and the phone on a real Wi-Fi, by the node's `.local` name, is the
  field night's check (plan.md § Outside the factory).
- **The scope node's computer is the owner's.** Whoever can read its state folder can
  read the token; the console guards the network, not the machine.
- **Out of scope:**
  - user accounts, and a list of paired devices;
  - access from outside the local network, and TLS (owner decision 5);
  - showing the token on Setup or as a QR code (owner decision 3);
  - the scope node's service file, any firewall and the node's other ports (a rig change,
    plan.md § Outside the factory);
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
§ Plan traps 1): the gate; the prompt and its cookie; MCP, the binary's token and the
browser test; the words. Each merges after the one before (`depends_on_merged`): US2 adds
the prompt to US1's `pairing` module and its tests use US1's rig; US3 adds the token file
and MCP's check to the same module, and its browser test needs the binary's token, which
US3 makes; US4's pages describe what US3 makes, and its test joins the `console` binary's
`mod` lines after US2's. Plain `depends_on` would order them without putting the earlier
code in the later story's base.
