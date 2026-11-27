# Implementation Plan: The Console Opens to a Paired Phone

**Branch**: `056-console-opens-to-a-paired-phone` | **Date**: 2026-11-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/056-console-opens-to-a-paired-phone/spec.md`

## Summary

A new module, `crates/finder/src/pairing.rs`, holds the console token and one axum
middleware, the gate, that `finder::router` puts in front of every route. A request from
a loopback peer passes untouched. Any other peer must name an allowed host (012's
`host_allowed`) and present the token, as `Authorization: Bearer` or as the
`console_token` cookie, compared without case, dashes or spaces through 013's
`scope_link::tokens_match`; `POST /api/stop` always passes; wrong tokens are counted per
peer address, 10 in 60 s and then 429. Page paths answer an unpaired device with the
console's own pairing prompt, a red-on-black HTML form posting to `POST /pair`, which
sets the cookie for 400 days; a wrong cookie is cleared. MCP's door asks the same, after
its stops. The binary reads the token from `console-token` in its state folder, or writes
a new one there (owner-only, 12 symbols of Crockford's base32 from `/dev/urandom`), and
never prints it. The React page, `dist`, `Cargo.toml` and `Cargo.lock` do not change.
Four stories: the gate; the prompt and its cookie; MCP and the binary's token; the words.

## Technical Context

**Language/Version**: Rust, the factory's toolchain; TypeScript for one Playwright test
(`crates/finder/web/e2e/`).

**Primary Dependencies**: `axum` 0.8.9 as the crate has it (`middleware::from_fn_with_state`,
the `ConnectInfo` extractor, which also reads `MockConnectInfo`), `scope-link`'s
`tokens_match`, the standard library's `/dev/urandom` read. No crate is added; no feature
of a crate changes.

**Storage**: `console-token` in the state folder, one line, mode 0600. The wrong-token
count lives in memory.

**Testing**: in-process router tests with `MockConnectInfo` peers (`console/` and
`mcp/` test binaries), the built binary (`cli/`), and one Playwright test in the default
project against the release binary on 127.0.0.1:8097.

**Target Platform**: the console on the scope node's computer (Linux on AArch64), a desk
console on Linux or macOS; `/dev/urandom` exists on all of `deny.toml`'s three targets.

**Constraints**: loopback answers byte-for-byte as at f6d4f5fd (FR-002); Stop answers
everyone (014, 017); no route ever answers the token; the gates make no network request.

**Scale/Scope**: one module of about 500 lines, three lines in `api.rs`, three in the
door, ten in `main.rs`, five test files, a README section and two glossary entries.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing of the solver, detection or numerics changes. The gate's behaviour is grounded in its cited sources: RFC 6265 (cookies, `HttpOnly`, `SameSite`), RFC 6750 (`Authorization: Bearer`, `WWW-Authenticate`), 012's same-origin and host rules, 013's token comparison, 014's and 017's Stop. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. The only edits to existing test files are the `mod` lines of the `console`, `mcp` and `cli` test binaries (SC-002). Every existing test passed unchanged on the probe's tip, 1,544 of 1,544 with the stories' tests (run, § What was run). |
| III. Tests first, independent expectations | The oracles are the answers a loopback peer and the plain console give, the scenarios' words, status codes and headers, the alphabet and the file's mode, and DESIGN.md's night tokens. No test reads `WRONG_TOKENS`, `WRONG_WINDOW_S` or `COOKIE_MAX_AGE_S`. Time comes from a clock the test sets. Every story's tests failed on the story before it (run). |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. The gate is a security path: its reason is spec.md's input, its diff is one module and three call sites, and every refusal and every pass has a test that fails without it (§ What was run, the mutations). |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download), in `~/.cargo/registry` and the repository's `reference-solutions/`: **`tower-http`** (its `ValidateRequestHeaderLayer::bearer` and CORS): not in the registry or `Cargo.lock`, and a bearer layer alone covers neither a cookie, a loopback exemption, Stop, a prompt nor a limit; **`axum-extra`'s cookie jar** and **`cookie`** 0.18.1 (in the registry, not in `Cargo.lock`): a cookie is one `Set-Cookie` line and one `split(';')`, not worth a dependency change; **`tower_governor` / `governor`** (rate limiting): not in the registry; a 60 s window per address is 20 lines, after 016's own `MoveLimit` (`crates/finder/src/mcp/limit.rs`); **`getrandom`** 0.4.3 (in `Cargo.lock` through `rmcp`, MIT OR Apache-2.0): owner decision 2 (b), turned down because a direct dependency is a `Cargo.toml` and `Cargo.lock` change for 12 bytes the standard library reads from `/dev/urandom` on every target; **`rustls`** 0.23.46 (in the registry, not in `Cargo.lock`) and a certificate generator (none found): owner decision 5 (b); a **QR encoder**: none in the registry or the page's `node_modules` (whose one cookie package, `tough-cookie`, is jsdom's client jar), owner decision 3 (c). A search of `reference-solutions/` for `Set-Cookie`, `WWW-Authenticate` and bearer tokens found nothing. So the gate is written fresh, on the console's own host rule, 013's comparison and 016's door. |

## Project Structure

### Documentation (this feature)

```text
specs/056-console-opens-to-a-paired-phone/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/finder/src/pairing.rs                    # new (US1, the gate); US2 the prompt and cookie; US3 the token file and MCP's check
crates/finder/src/api.rs                        # US1: the `pairing` field, `router` split; US2: `pairing::routes()`
crates/finder/src/lib.rs                        # US1: `pub mod pairing;` and a doc paragraph; US2: the `/pair` line
crates/finder/src/mcp/door.rs                   # US3: three lines after the peer check
crates/finder/src/main.rs                       # US3: the token file and the stderr line
crates/finder/README.md, GLOSSARY.md            # US4
crates/finder/tests/console/pairing.rs          # US1, new (its helpers pub(super) for US2)
crates/finder/tests/console/pairing_prompt.rs   # US2, new
crates/finder/tests/console/pairing_docs.rs     # US4, new
crates/finder/tests/mcp/pairing.rs              # US3, new
crates/finder/tests/cli/pairing.rs              # US3, new
crates/finder/web/e2e/pairing.spec.ts           # US2, new (default project; no config change)
```

**The other drafts' files.** 055 writes `crates/finder/src/api.rs` (a field before
`move_limit`, one `merge` before `routes.with_state(state)`) and `crates/finder/src/lib.rs`
(`pub mod nudge;`); 052 to 055 all write `crates/finder/README.md`, `GLOSSARY.md` and the
test binaries' `main.rs` files. This spec keeps out of their lines: its `AppState` field
follows `allowed_hosts`, its router change is at the top of `router`, its module line is
after `pub mod imaging;`, and its README section sits inside `### From a phone`. They
land in the order 052, 053, 054, 055, and the seat re-anchors this trio on that `main`
before any flip (§ Prerequisites).

## Interfaces

### `crates/finder/src/pairing.rs` (US1 new; US2 and US3 add)

Module doc: what spec.md's FR-001 to FR-005 say, in the module's own words, and (US3)
that the binary keeps the token in `console-token`.

**US1:**

- `pub const COOKIE: &str = "console_token";`
- `pub const WRONG_TOKENS: usize = 10;` and `pub const WRONG_WINDOW_S: f64 = 60.0;`
- `const MAX_PEERS: usize = 1024;`
- `pub struct ConsoleToken { normal: String }`, `#[derive(Clone)]` only, never `Debug` or
  `Serialize`. `fn normalize(text: &str) -> String` drops `-` and white space and
  upper-cases ASCII. `pub fn new(text: &str) -> Result<ConsoleToken, InvalidToken>`
  accepts a normalized 8 to 64 ASCII letters and digits. `fn matches(&self, supplied:
  &str) -> bool` is `scope_link::tokens_match(normalize(supplied).as_bytes(),
  self.normal.as_bytes())`.
- `pub struct InvalidToken;` (`Debug`, `Display` = `a console token is 8 to 64 letters
  and digits; dashes and spaces are ignored`, `std::error::Error`).
- `pub(crate) struct Pairing { token: ConsoleToken, wrong: Mutex<HashMap<IpAddr,
  VecDeque<f64>>> }` (a `std::sync::Mutex`, never held across an await) and `fn
  check(&self, peer: IpAddr, supplied: &str, now: f64) -> Check`: first drop every time
  at least `WRONG_WINDOW_S` old and every emptied address; then `Limited` if the address
  holds `WRONG_TOKENS` times; `Admitted` if the token matches; `Limited` if the address is
  new and `MAX_PEERS` are held; else record `now` and `Wrong`.
- `impl AppState { pub fn with_console_token(mut self, token: ConsoleToken) -> Self }`,
  setting `api.rs`'s new `pub(crate) pairing: Option<Arc<Pairing>>`.
- `fn loopback(ip: IpAddr) -> bool`: IPv4 `is_loopback`, IPv6 `is_loopback` or an
  IPv4-mapped loopback.
- `async fn peer(parts: &mut Parts) -> Option<IpAddr>`:
  `ConnectInfo::<SocketAddr>::from_request_parts(parts, &())`, the extractor, because
  `MockConnectInfo` stores itself, not a `ConnectInfo`, in the extensions (plan trap 3).
- `fn presented(headers: &HeaderMap) -> Option<(String, bool)>`: an `Authorization`
  header gives its text after `Bearer ` (or the empty string, so `Basic` counts as
  wrong), with `false`; else the first `console_token=` pair of any `Cookie` header split
  on `;` and trimmed, with `true` (from the cookie).
- `const UNPAIRED`, `WRONG`, `LIMITED: (&str, &str)`: the prompt's words (US2) and the
  API's words: `("Pair this device: enter the console token.", "this device is not
  paired: open the console's page and enter the console token")`, `("That is not the
  console token.", "that is not the console token")`, `("Too many wrong console tokens
  from this device: wait a minute and try again.", "too many wrong console tokens from
  this device: wait a minute")`.
- `pub(crate) async fn gate(State(state): State<AppState>, request: Request, next: Next)
  -> Response`: no `pairing` → `next`; split the request; a loopback peer, or the path
  `/api/stop` or `/mcp` → `next` (the door asks for `/mcp`, US3); `!api::host_allowed` →
  403 `ApiError` with BAD HOST's words; no token → 401; `Admitted` → `next`; `Wrong` →
  401; `Limited` → 429; each refusal an `ApiError` with the API's words. A request with
  no peer counts under `0.0.0.0`.

**US2 adds:**

- `pub const COOKIE_MAX_AGE_S: u64 = 400 * 24 * 60 * 60;` and `const FORM_LIMIT: usize
  = 1024;`
- In `gate`: the path `/pair` → `next` after the host check; a refusal on a path not
  under `/api/` is `(status, prompt_html(words))`; a `Wrong` whose token came from the
  cookie adds `Set-Cookie: console_token=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict`.
- `pub(crate) fn routes() -> Router<AppState>`: `Router::new().route("/pair",
  get(prompt).post(pair))`. `prompt` answers 200 with the prompt and UNPAIRED's words.
  `pair` runs `api::check_origin` first (its 403 as it is); without a token on the
  console, 404 `this console asks for no token`; reads at most `FORM_LIMIT` bytes (413
  past it); takes `form_field(body, "token")` (`&`-split pairs, `+` as space, `%XX`
  decoded) or the empty string, and `check`s it under the peer's address: `Admitted` →
  303, `Location: /`, `Set-Cookie: console_token=<normal>; Path=/;
  Max-Age={COOKIE_MAX_AGE_S}; HttpOnly; SameSite=Strict`; `Wrong` → 401 prompt;
  `Limited` → 429 prompt.
- `fn prompt_html(message: &str) -> Html<String>`, exactly this page with `{message}`
  in place (FR-010; US2-S4 reads its text):

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<title>Pair with the finder console</title>
<style>
html, body { margin: 0; background: #000000; color: #e00000; font: 540 16px/1.5 system-ui, sans-serif; }
main { max-width: 24rem; margin: 0 auto; padding: 2rem 1rem; }
h1 { font-size: 1.25rem; font-weight: 600; margin: 0 0 1rem; }
p { color: #d40000; }
label { display: block; margin: 1rem 0 0.25rem; }
input, button { box-sizing: border-box; width: 100%; min-height: 44px; border-radius: 0; font: inherit; color: #e00000; caret-color: #e00000; }
input { padding: 0 0.5rem; background: #000000; border: 1px solid #a80000; letter-spacing: 0.1em; text-transform: uppercase; }
button { margin-top: 1rem; background: #5a0000; border: 1px solid #e00000; }
input:focus, button:focus { outline: 2px solid #e00000; outline-offset: 2px; }
::selection { background: #5a0000; color: #e00000; }
.note { color: #a80000; }
</style>
</head>
<body>
<main>
<h1>Pair with the finder console</h1>
<p role="status">{message}</p>
<form method="post" action="/pair">
<label for="token">Console token</label>
<input id="token" name="token" autocomplete="off" autocapitalize="characters" spellcheck="false" required>
<button type="submit">Pair</button>
</form>
<p class="note">The console token is in the file console-token in the console's state folder. This browser keeps it until the token changes.</p>
</main>
</body>
</html>
```

  (In Rust, a raw string `r##"..."##` through `format!`, the CSS braces doubled.)

**US3 adds:**

- `pub const TOKEN_FILE: &str = "console-token";` and `const ALPHABET: &[u8; 32] =
  b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";`
- `pub fn generate() -> io::Result<ConsoleToken>`: 12 bytes `read_exact` from
  `/dev/urandom`, each `b & 31` into `ALPHABET` (256 is a multiple of 32, so uniform).
- `pub fn grouped(&self) -> String`: the normal form in groups of four joined by `-`.
- `pub fn load_or_create(path: &Path) -> Result<ConsoleToken, String>`: read the file and
  `new` it, the error `"{path}: {InvalidToken}"`; if it does not exist, `generate`, and
  write `grouped()` and a newline with `OpenOptions` `write`, `create_new` and, on unix,
  `mode(0o600)`, creating the folder first; other read errors `cannot read {path}:
  {error}`. Never the contents in an error.
- `pub(crate) async fn admits_mcp(parts: &mut Parts, state: &AppState) ->
  Option<Response>`: `None` with no `pairing`, for a loopback peer or the right token;
  else `Some` of `(status, [(WWW-Authenticate, "Bearer realm=\"finder console\"")],
  Json({"jsonrpc": "2.0", "error": {"code": -32001, "message": M}}))` with 401 and `MCP
  from another device needs the console token as a bearer token` (no token), 401 and
  WRONG's API words, or 429 and LIMITED's API words. (Returning `Option`, not `Result`:
  clippy's `result_large_err` refuses a `Response` error.)

### `crates/finder/src/api.rs` (US1; US2 one word)

- `AppState` gains `pub(crate) pairing: Option<Arc<crate::pairing::Pairing>>` after
  `allowed_hosts` (`crates/finder/src/api.rs:115`), `None` in `shared` after
  `allowed_hosts: Arc::new(Vec::new()),` (`crates/finder/src/api.rs:155`).
- `router` (`crates/finder/src/api.rs:616`) becomes:

```rust
/// The console's routes, behind the console token's gate (056).
pub fn router(state: AppState) -> Router {
    console_routes(state.clone()).layer(axum::middleware::from_fn_with_state(
        state,
        crate::pairing::gate,
    ))
}

fn console_routes(state: AppState) -> Router {
    let mut routes = Router::new()
```

  and the rest of the old body unchanged. US2 changes that `Router::new()` to
  `crate::pairing::routes()`. `Router::layer` after every `merge` and the fallback puts
  the gate before every route, the page and `/mcp`'s door among them.

### `crates/finder/src/lib.rs` (US1; US2 one line)

US1: after `pub mod imaging;` (`crates/finder/src/lib.rs:115`), a blank line,
`/// The console token's gate for devices other than the console's own machine (056).`
and `pub mod pairing;`; after the paragraph ending `[`AppState::with_allowed_hosts`].`,
`//!` and: "With a console token ([`AppState::with_console_token`]), a peer that is not
loopback must present it, as the `console_token` cookie or a bearer header, and name an
allowed host before any route but `POST /api/stop` answers ([`pairing`])." US2 adds
`/pair` and before `POST /api/stop` there, and after the route list's last line
`//! * GET /pair: the pairing prompt; POST /pair: a form with the console token, answered
303 with its cookie (056).`

### `crates/finder/src/mcp/door.rs` (US3)

After `if !peer_allowed { return denied(); }` (`crates/finder/src/mcp/door.rs:64`):

```rust
    if let Some(refusal) = crate::pairing::admits_mcp(&mut parts, &state).await {
        return refusal;
    }
```

The stops (`crates/finder/src/mcp/door.rs:26`) stay first; 016's checks stay as they are.

### `crates/finder/src/main.rs` (US3)

After `let session = Session::with_state_file(...)?;` (`crates/finder/src/main.rs:193`):
`let token_path = state_dir.join(finder::pairing::TOKEN_FILE);` and `let token =
finder::pairing::ConsoleToken::load_or_create(&token_path)?;`; `.with_console_token(token)`
after `.with_allowed_hosts(args.allow_host)` (`crates/finder/src/main.rs:199`); and after
the loopback hint (`crates/finder/src/main.rs:277` to `:279`) an `else` printing
`other devices pair with the console token in {token_path.display()}`. `--listen`'s
default and doc stay. A bad file returns the error from `main`, so the start fails before
it binds.

### The README (`crates/finder/README.md`, US4)

The `--listen` row (`crates/finder/README.md:34`) becomes:

```text
| `--listen ADDR` | 127.0.0.1:8080 | Address to serve on. 0.0.0.0:8080 lets other devices connect; they must present the console token (§ The console token). |
```

After the location paragraph (`crates/finder/README.md:61` to `:63`), a blank line and:

```markdown
### The console token

Every device other than the console's own machine must present the console token
before the console answers it. At its first start the console writes a new token,
twelve letters and digits in three groups such as `ABCD-EFGH-JKMN`, to `console-token`
in its state folder, readable by its owner only; it never prints the token. Read it
there, for example over ssh with `cat ~/.local/state/finder/console-token`, or in the
folder the console's `--state` names.

Open http://<the console's address>:8080 on the phone and it asks for the console
token. Type it once; case, dashes and spaces do not matter. The browser keeps it in a
cookie for 400 days, and every page, live stream and picture then works as it does on
the console's own machine. Another device's MCP client, or curl, sends it as
`Authorization: Bearer <token>`.

- Clients on the console's own machine are never asked, an ssh tunnel to
  127.0.0.1:8080 among them.
- `POST /api/stop` and MCP's `stop` and `stop_all` stop the mount from any device,
  paired or not.
- A device that sends 10 wrong tokens within 60 s is answered 429 until the oldest of
  them is a minute old.
- Another device must open the console at an IP address, `localhost` or a name given
  with `--allow-host`.
- To change the token, stop the console, delete `console-token` or write your own (8 to
  64 letters and digits) in its place, and start the console again; every paired
  browser then asks again.
- The console speaks plain HTTP, so the token crosses the Wi-Fi unencrypted: use it on
  a private network you trust, such as a phone's own hotspot, never on a public one.
```

In `## MCP clients`, `There is no token or console confirmation for MCP calls. Every
moving tool runs` (`crates/finder/README.md:80`) becomes:

```text
An MCP client on the console's own machine needs no token; another device's client
sends the console token as `Authorization: Bearer <token>` (§ The console token).
There is no console confirmation for MCP calls. Every moving tool runs
```

### The glossary (`GLOSSARY.md`, US4)

**MCP client** (`GLOSSARY.md:265` to `GLOSSARY.md:267`) takes FR-014's words, its `_Avoid_` line
unchanged, then a blank line and **Console token** with FR-014's words and `_Avoid_:
password, PIN, API key, node token`.

### The browser test (`crates/finder/web/e2e/pairing.spec.ts`, US2)

`test.use({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 })`; one test
named `US2-S5: ...`; `page.goto("/pair")`, `getByRole("heading", ...)`,
`getByLabel("Console token")`, `getByRole("button", { name: "Pair" })`, one
`page.evaluate` collecting the six computed colours of `document.documentElement` and
`body, body *` (skipping `rgba(0, 0, 0, 0)`), each matched against
`/^rgba?\((\d+), 0, 0(?:, [\d.]+)?\)$/`, `toHaveCSS` for `html`'s background and the
focused field's outline, `boundingBox()` heights, then `fill` and `click` and
`getByRole("status")`. Its file name is not in the default project's `testIgnore`
(`crates/finder/web/playwright.config.ts:36` to `crates/finder/web/playwright.config.ts:46`), so it runs there with no
config change, against 127.0.0.1:8097.

## Opening the console from a phone

What the owner does, written for the README and for § Outside the factory:

1. Start the console listening on the network (--listen 0.0.0.0:8080); its stderr says
   where the token is.
2. Read the token from that file over ssh, or ask the Claude session to.
3. On the phone, on the same Wi-Fi, open http://<the console's address>:8080, type the
   token into the prompt, press Pair. The page opens and stays paired for 400 days.
4. The tunnel and Claude keep working as before.

## What was run and what was read

All on the probe branch `probe/056`, cut from f6d4f5fd, on the factory's toolchain with
`--locked`, npm from its cache, the browsers from the factory's Playwright cache. Logs are
outside the repository.

**The stories' commits.** ef9009de (US1); fcc4bd2d (US2) with its fold e5f3c146 (the
browser test's name); 891e1805 (US3 and US4 together), split by a0447d0c (PROBE ONLY:
US4's files backed out, leaving US3) and 9061c9d8 (US4), with the folds 4e068469 (US3:
US3-S1's calls in the scenario's order), 7836bd15 (US4: the README read with white space
as one space; the glossary's two entries in order) and b2513833 (US4: the tunnel's
sentence and the `--listen` row's end, as FR-013 words them). The tip is b2513833.

- **The gates.** `lint` passed (fmt, clippy with `-D warnings`, both doc builds) at
  e5f3c146. The `test` gate's cargo half at e5f3c146: nextest 1,544 of 1,544 (1,525
  existing and the stories' 19), the doc tests passing. Its page half at 7836bd15:
  typecheck, the Prettier check, Vitest 607 of 607, the build, `dist` unchanged, and
  `CI=1 npm run e2e` 35 of 35, `pairing.spec.ts` among them. The two later folds change
  only `pairing_docs.rs` and `mcp/pairing.rs`, whose tests passed after each.
- **Every story's tests failed on the story before.** US1's do not build on f6d4f5fd (no
  `finder::pairing`); US2's 4 Rust tests failed on US1's code (no `/pair`, no prompt, no
  cookie); US3's and US4's 6 failed on US2's code: the two MCP tests (no token asked),
  the two docs tests, the first-start test (no `console-token` written), and the
  bad-token test ran into nextest's 300 s limit, the console starting anyway, which is
  why US3-S4 now waits at most 60 s itself.
- **Eight mutations, each failing its scenario's test** (one at a time, restored after):
  a request with no peer read as loopback (US1-S2); no Host check (US1-S5); Stop behind
  the gate (US1-S5); one wrong-token count for every peer (US1-S4); a wrong cookie kept
  (US2-S3); `POST /pair` from any origin (US2-S3); the token file at mode 0644 (US3-S3);
  IPv6 never loopback (US1-S1).
- **Loopback is unchanged (US1-S1).** With a token, 127.0.0.1, ::1 and
  ::ffff:127.0.0.1 got the plain console's status, content type and bytes for the six
  reads, with each Host and none, and `PUT` 200.
- **The cookie reaches the page's every request.** A paired phone's `GET /`, `/api/live`
  (200, `text/event-stream`) and both previews matched loopback's with the cookie alone
  (US1-S3, US2-S2), which is what the React page sends with no change.
- **`MockConnectInfo` is not `ConnectInfo`.** The first gate read
  `extensions().get::<ConnectInfo<SocketAddr>>()` and saw no peer in every test, so every
  loopback test answered 401; the extractor reads both (plan trap 3).
- **The limit.** Ten wrong tokens at T to T + 9 s: the right cookie 429 at T + 59.9 s
  and 200 at T + 60 s; the other phone 200 throughout (US1-S4).

Read, not run: the phone on a real Wi-Fi and the scope node's service (§ Outside the
factory); WSL2 NAT and portproxy peers (Edge Cases, from the README's own section); a
real phone browser's handling of a self-signed certificate (owner decision 5 (b)) and
of a cookie's 400 days (owner decision 4), from memory, not read here, with no network;
RFC 6265 and RFC 6750, cited from memory. The binary was started only on loopback and
0.0.0.0, reached through 127.0.0.1.

## Read from the diff, not from a test

- The gate is the last layer of `router`, after every `merge` and the fallback.
- No route returns the token; `ConsoleToken` has no `Debug`, `Display` or `Serialize`.
- `Pairing`'s mutex is never held across an await.
- The door's order is stops, 016's checks, then the token.
- `main.rs` loads the token before it binds, so a bad file stops the start before the
  `finder console:` line.

## Shapes that existing tests pin

- **016's door.** `crates/finder/tests/mcp/endpoint.rs:202` builds `app(state(), peer)`
  with no console token and expects LAN peers 200 and public peers 403: unchanged,
  since a console without a token asks nothing (FR-006) (run).
- **012's same-origin rule.** `crates/finder/tests/console/session/writes.rs` sends
  writes with and without `Origin` from routers with no peer and no token: unchanged
  (run).
- **Routers with no peer.** Of the 66 test files that name `router(`, 4 mock a peer;
  none sets a console token, so FR-001's "no peer is not loopback" never meets them (run).
- **The binary's address line.** `crates/finder/tests/cli/session.rs:58` reads
  `finder console: http://`; it and the loopback hint stay first (run).
- **The README's MCP count and sections.** `The imaging camera has sixteen tools of its
  own` (`crates/finder/README.md:88`) and `## MCP clients` / `## API` stay, so
  `crates/finder/tests/console/imaging_run_docs.rs:138` and
  `crates/finder/tests/mcp/finder_focus.rs` pass (run).
- **The browser specs on 8097.** The default project's specs open the page over
  loopback, so the gate never asks them; `pairing.spec.ts`'s wrong token counts under
  127.0.0.1 only at `/pair`.

## Decisions and the alternatives rejected

| Decision | Chosen | Rejected, and why |
|---|---|---|
| Where the gate sits | one middleware on `router`, after every route | per-route extractors (every route, and the next route, would need one) |
| How a browser presents it | a cookie set by the prompt | a header on every `fetch` (37 calls; `EventSource` and `<img>` cannot) |
| The prompt | the console's own HTML, always night | the React page (its files open to unpaired devices; `dist` changes) |
| MCP | the door asks after its stops | the gate before the door (would refuse a stop: 016's FR-012) |
| The token's source | the state folder's file, generated once | a flag (process list), a PIN (guessable), per start (re-pair nightly) |
| Randomness | `/dev/urandom` with the standard library | `getrandom` (a `Cargo.lock` change for the same bytes) |
| The limit | per address, sliding 60 s | one count for all (a device on the Wi-Fi keeps the owner out) |
| A wrong cookie | cleared in the answer | kept (every request of an open page would count against the limit) |
| `--listen` | unchanged | `0.0.0.0` by default |

## User Story 1 - The gate

`pairing.rs` with the token, the count and the gate; `api.rs`'s field and `router`;
`lib.rs`'s module line and paragraph; the tests of spec.md US1.

## User Story 2 - The prompt and its cookie

`/pair`, the prompt, the cookie set and cleared; `api.rs`'s `routes()`; `lib.rs`'s
line; the tests of spec.md US2 and the browser test.

## User Story 3 - MCP and the binary's token

`admits_mcp` and the door's three lines; `generate`, `grouped`, `load_or_create`;
`main.rs`; the tests of spec.md US3.

## User Story 4 - The words

The README's row, section and sentence; the glossary; the tests of spec.md US4.

## Verification (every story, before proposing)

The gates, copied from `ergane.yaml`:

- `lint`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  --all-features --locked -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc
  --workspace --no-deps --all-features --locked` and `RUSTDOCFLAGS="-D warnings" cargo
  doc -p finder --no-deps`.
- `test`: `cargo nextest run --workspace --all-targets --all-features --locked`,
  `cargo test --workspace --locked --doc`, then in `crates/finder/web`: `npm ci
  --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`, `npm test`, `npm
  run build`, `git status --porcelain -- dist` empty, and `CI=1 npm run e2e`.

## Prerequisites before this spec is flipped to `ready`

- 052, 053, 054 and 055 have landed, in that order, and this trio is re-anchored on that
  `main`: every `path:line` read again; `api.rs`'s field and `router` change placed
  against 055's field and `merge`; `lib.rs`'s module line after 055's; the README section,
  the glossary entries and the test binaries' `mod` lines placed after theirs; every
  story's size measured again; the Refinement Record's last writers read again. Waits
  for 052 to 055 and the seat's re-anchor.
- The owner's nine decisions are answered. Waits for the flip ask.
- A probe round from this text alone, folded. Waits: only the drafting probe has run.
- The page half has run once on the re-anchored base (`CI=1 npm run e2e`). Waits for the
  re-anchor.
- No operator-staged file is needed. Met: every story writes only code, tests and docs.

## Outside the factory

Written here and not run by any gate.

**The rig change.** The scope node's service for the console adds `--listen
0.0.0.0:8080` to its command line and is restarted; its state folder then holds
`console-token` after the first start. If the scope node's computer runs a firewall,
allow TCP 8080 from the local network only. Nothing else on the rig changes: the ssh
tunnel to 127.0.0.1:8080 and the Claude session work as before.

**On the next field night**, with the phone on the scope's Wi-Fi: read the token from
the state folder over ssh; open http://<the scope node's address>:8080 on the phone;
type the token; check that the page opens, the live view runs and the night switch
works; press one nudge arrow (055) and Stop; note whether anything asked for the token
again. Then check the tunnel's page and an MCP call from the laptop still work with no
token. Notes go in the night's report; nothing in the repository waits for them.

## Plan traps

They bind every story.

1. **The story sizes.** The probe's commits, each diff from the previous story's tip,
   folds added: US1 22,672 B (34.6 % of 65,536; 30,607 B at 1.35 times, 77.8 % of
   39,321); US2 22,027 + 653 = 22,680 B (34.6 %; 30,618 B, 77.9 %); US3 22,251 + 1,240
   = 23,491 B (35.8 %; 31,713 B, 80.7 %), 22,251 B written once; US4 9,021 + 5,104 +
   1,564 = 15,689 B (23.9 %; 21,180 B, 53.9 %), 9,295 B written once. Every story is
   under 39,321 B at 1.35 times. The draft's three stories put US4's words in US3
   (31,272 B; 42,217 B at 1.35 times, over the line), so the words are a story of their
   own.
2. **An earlier story's test file is an existing test.** US2's tests use US1's helpers
   from `super::pairing`, which US1 writes `pub(super)` from the start, each used by
   US1's own tests (an unused `pub(super)` item is dead code to clippy): `TOKEN`,
   `PHONE`, `OTHER_PHONE`, `HOST`, `ORIGIN`, `COOKIE`, `UNPAIRED`, `WRONG`, `LIMITED`,
   `READS`, `StepClock`, `clock`, `gated`, `from`, `get`, `send`, `json_of`, `answer`.
   No story edits another's test, the unit tests in `pairing.rs` among them: US3 adds its
   own `#[test]` functions there.
3. **`MockConnectInfo` stores itself.** Read the peer with the `ConnectInfo` extractor
   (`from_request_parts`), never with `extensions().get::<ConnectInfo<_>>()`.
4. **An unused item fails clippy.** `admits_mcp` arrives with the door (US3), not
   before; `COOKIE_MAX_AGE_S` and `FORM_LIMIT` with `/pair` (US2).
5. **`Result<(), Response>` fails clippy.** `admits_mcp` returns `Option<Response>`.
6. **A start that should fail may not.** US3-S4 waits at most 60 s for the bad-token
   start to exit and kills it after; a plain `output()` would wait for a console that
   started anyway.
7. **The token is never in a test's output.** Tests use `test-console-token`
   (low entropy, for `ripsecrets`), and US3 asserts no stderr and no call log holds it.
8. **The default console is shared.** `pairing.spec.ts` opens only `/pair` on 8097 and
   sends one wrong token from 127.0.0.1, which the gate never limits on other routes.
9. **rustdoc denies warnings.** No doc comment holds an angle bracket outside
   backticks (`Authorization: Bearer <token>` inside backticks).
10. **Each worktree builds on its own.** No story sets `CARGO_TARGET_DIR`.
11. **"Finder" alone is the glossary's _Avoid_.** The prompt says "the finder
    console", never "the finder".

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| A device on the Wi-Fi sniffs plain HTTP | it learns the token | the README says private network only; TLS is a later decision (5 (b)) |
| The owner loses the token | the phone cannot pair | the tunnel still works; read or replace the file |
| A browser keeps the cookie less than 400 days | the phone asks again | one prompt |
| WSL2 NAT peers are not loopback | a Windows browser needs the token | the README's WSL2 section; mirrored networking keeps loopback |
| 055 changes `api.rs` and `lib.rs` | the lines move | placed away from 055's; re-anchored before the flip |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, from a probe of that day on `probe/056` (four stories built with every
scenario's test, both gates passing, every story's tests failing on the story before,
eight mutations biting), and a reading of the router, MCP's door,
012's same-origin and host rules, 013's token comparison, 014's and 017's Stop, the
binary's start, the page's requests, the constitution, the glossary and the ADRs. Writes
come from the task text. Shared writes name the spec story that last wrote the same file
on `main`; within the spec, each story's files written by an earlier one are ordered by
`depends_on_merged`.

None of the nine owner decisions of spec.md is answered; this record is written to the
recommendations. Its `open_questions` are empty because the validator refuses a record
that holds any; what is open is spec.md's owner decisions 1 to 9 and § Prerequisites
(the re-anchoring after 052 to 055). Nothing in this record is an owner approval: those
are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: f6d4f5fd482ccc9d5b634ef1aa5124c112769583
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "the gated console (support mcp state, a set clock, console token test-console-token) and the plain console: each read (GET /api/session, /api/health, /api/live, /api/live/preview.jpg?seq=1, /api/imaging/frames/1/preview.jpg, /) from 127.0.0.1, ::1 and ::ffff:127.0.0.1 with Host localhost:8080, Host 127.0.0.1:8080 and no Host answers the status, content type and body the plain console gives 127.0.0.1 (event-stream: status and type); PUT /api/session {target M42} from each answers 200"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "the phone 192.168.1.30 with Host 192.168.1.20:8080 and no token: each read, GET /setup, GET /assets/index.js and PUT /api/session answer 401, those under /api/ exactly {error: this device is not paired: open the console's page and enter the console token}; GET /api/session through finder::router with no peer recorded the same 401; the session read from 127.0.0.1 after equals the one before"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "from the phone with Cookie console_token=TESTCONSOLETOKEN, with Cookie theme=night; console_token=test-console-token, with Authorization Bearer test-console-token and with Bearer TESTCONSOLETOKEN: each read's answer equals the one 127.0.0.1 gets with Host 192.168.1.20:8080 and no token; PUT /api/session {target M42} with the cookie and Origin http://192.168.1.20:8080 answers 200 and target.messier is M42"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "GET /api/health from the phone at T + 0..9 s with Cookie console_token=NOPE-NOPE, Bearer NOPE-NOPE, Basic dGVzdA== and seven Bearer NOPE-NOPE: each 401 exactly {error: that is not the console token}; at T + 59.9 s the phone's GET /api/session with the cookie 429 exactly {error: too many wrong console tokens from this device: wait a minute} and its GET / with the bearer 429, the other phone 192.168.1.31's 200; at T + 60 s the phone's 200"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "with_allowed_hosts finder.example: POST /api/stop from the phone with Host rebound.example:8080 and no token answers 200 exactly {stopped: true}; GET /api/session with the cookie and Host rebound.example:8080, Host rebound.example and no Host answer 403 exactly {error: another device must open the console at an IP address, localhost or an allowed host}; with Host finder.example:8080 200"
        decision: "spec.md:US1-S5"
      - path: crates/finder/src/pairing.rs
        status: new
        disposition: add
        assertion: "unit: abcd-efgh-jkmn matches ABCDEFGHJKMN and ' abcd efgh jkmn ', not ABCDEFGHJKMP or ABCDEFGHJKM; short, not/a/token and 65 letters are refused by ConsoleToken::new"
        decision: "spec.md:FR-001"
      - path: crates/finder/tests/mcp/endpoint.rs
        status: existing
        disposition: preserve
        assertion: "016's door with no console token: LAN peers 200, public peers and no peer 403, Origin and Host refusals as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/session/writes.rs
        status: existing
        disposition: preserve
        assertion: "012's same-origin rule on session edits and solves, from routers with no peer and no console token"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a console without a console token answers every request as at f6d4f5fd (FR-006)"
      - "POST /api/stop answers every peer as at f6d4f5fd (014 owner approval 2, 017 FR-004, FR-006)"
      - "012's check_origin and host_allowed are called, not changed; /mcp's door is not changed in US1"
    checks:
      development:
        - "cargo nextest run -p finder --locked --lib --test console -E 'test(/^pairing::/)'"
        - "cargo nextest run -p finder --locked --test mcp -E 'test(endpoint::)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/pairing.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/pairing.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "056 depends_on_landed 051, whose US2 last changed api.rs (f9706cc0)"
      - path: crates/finder/src/lib.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US1
        disposition: merge_order
        reason: "056 depends_on_landed 051, whose US1 last changed lib.rs (4c39350e)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US2
        disposition: merge_order
        reason: "056 depends_on_landed 051, whose US2 last changed console/main.rs (f9706cc0)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/pairing_prompt.rs
        status: new
        disposition: add
        assertion: "the phone with no token: GET /, /setup and /assets/index.js answer 401, no Set-Cookie, text/html; charset=utf-8 holding <h1>Pair with the finder console</h1>, <p role=status>Pair this device: enter the console token.</p> and <form method=post action=/pair>; GET /pair the same with 200; GET /api/session 401 with the error this device is not paired: open the console's page and enter the console token"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/pairing_prompt.rs
        status: new
        disposition: add
        assertion: "POST /pair from the phone, form-urlencoded, Origin http://192.168.1.20:8080, token=test-CONSOLE+token: 303, Location /, Set-Cookie exactly console_token=TESTCONSOLETOKEN; Path=/; Max-Age=34560000; HttpOnly; SameSite=Strict; each read with that cookie's name=value equals 127.0.0.1's answer with no token; token=TEST%2DCONSOLE%2DTOKEN 303"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/pairing_prompt.rs
        status: new
        disposition: add
        assertion: "form NOPE-NOPE: the prompt That is not the console token. 401, no Set-Cookie; GET /api/session with Cookie console_token=NOPE 401 error that is not the console token, Set-Cookie exactly console_token=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict; GET / with it the prompt 401 and that Set-Cookie; GET / with Bearer NOPE the prompt 401, no Set-Cookie; six more forms 401; the right form and GET / with the cookie the prompt Too many wrong console tokens from this device: wait a minute and try again. 429, the form with no Set-Cookie; the other phone's form with Origin http://elsewhere.example 403 error writes require the console's own origin and an allowed host, no Set-Cookie"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/pairing_prompt.rs
        status: new
        disposition: add
        assertion: "GET /pair from the phone and 127.0.0.1: 200; at least 10 # in each page, each beginning #000000, #e00000, #d40000, #a80000 or #5a0000; none of rgb, hsl, white, <script, <img, <link, url(; holds <label for=token>Console token</label>, <input id=token name=token, min-height: 44px and html, body { margin: 0; background: #000000;"
        decision: "spec.md:US2-S4"
      - path: crates/finder/web/e2e/pairing.spec.ts
        status: new
        disposition: add
        assertion: "the default console at 412 x 915, scale 2: /pair 200 with the heading Pair with the finder console; with the field Console token focused, more than 10 computed colours of html, body and every element in body (color, background-color, border-top-color, border-bottom-color, outline-color, caret-color) other than rgba(0, 0, 0, 0), each rgb or rgba with green and blue 0; html background rgb(0, 0, 0); the field's outline rgb(224, 0, 0); field and button at least 44 px tall; after NOPE-NOPE-NOPE and Pair the status That is not the console token. under the same heading"
        decision: "spec.md:US2-S5"
      - path: crates/finder/src/pairing.rs
        status: new
        disposition: add
        assertion: "unit: form_field decodes token=AB+CD%2DEF to AB CD-EF, finds token in x=1&token=Q, and gives none for x=1"
        decision: "spec.md:FR-008"
    preserved_behavior:
      - "US1's answers under /api/ and its limit are unchanged; only page paths gain the prompt"
      - "the React page and dist are not changed; playwright.config.ts is not edited"
    checks:
      development:
        - "cargo nextest run -p finder --locked --lib --test console -E 'test(/^pairing_prompt::/) | test(/^pairing::tests::/)'"
        - "npx playwright test --project=default e2e/pairing.spec.ts"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs (finder-page among them)"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/pairing.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/pairing_prompt.rs
      - crates/finder/web/e2e/pairing.spec.ts
    no_write_reason: ""
    shared_writes: []
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/mcp/pairing.rs
        status: new
        disposition: add
        assertion: "the MCP rig: get_session and set_target M42 from the phone with no token 401, www-authenticate exactly Bearer realm=finder console, body exactly {jsonrpc 2.0, error {code -32001, message MCP from another device needs the console token as a bearer token}}; with Bearer NOPE-NOPE 401 message that is not the console token; the session unchanged; with the bearer get_session 200 structuredContent equal to the session and set_target 200, target.messier M42; 127.0.0.1 with no token 200; 203.0.113.7 with the bearer 403; the phone on a console without a token 200"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/mcp/pairing.rs
        status: new
        disposition: add
        assertion: "with the call log: stop from the phone with no token 401 with the no-token refusal and the log exactly mcp stop {} -> {\"stopped\":true} and a newline; ten get_session with Bearer NOPE-NOPE 401; then the bearer 429 message too many wrong console tokens from this device: wait a minute; GET /api/session from the phone with the cookie 429; the log in lower case holds no test-console-token"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/cli/pairing.rs
        status: new
        disposition: add
        assertion: "the built finder, an empty --state folder: started on 127.0.0.1:0 and stopped, console-token is three groups of four of 0123456789ABCDEFGHJKMNPQRSTVWXYZ joined by - and one newline, mode 0600; stderr holds (to open it from a phone, run with --listen 0.0.0.0:8080); started on 0.0.0.0:0, GET /api/session to 127.0.0.1 at its port answers HTTP/1.1 200 OK, stderr holds other devices pair with the console token in <the file's path>, the file unchanged; neither stderr holds the token with or without dashes"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/cli/pairing.rs
        status: new
        disposition: add
        assertion: "console-token my-own-console-token and a newline is kept as written and not printed; tiny7 and a newline: the start exits unsuccessfully within 60 s, stderr holds <the file's path>: a console token is 8 to 64 letters and digits; dashes and spaces are ignored, and neither tiny7 nor finder console: http://"
        decision: "spec.md:US3-S4"
      - path: crates/finder/src/pairing.rs
        status: new
        disposition: add
        assertion: "unit: grouped gives ABCD-EFGH-JKMN for abcdefghjkmn and MYOW-NCON-SOLE-TOKE-N for my-own-console-token; generate gives 12 symbols of the alphabet, two generations differ"
        decision: "spec.md:FR-012"
      - path: crates/finder/tests/cli/session.rs
        status: existing
        disposition: preserve
        assertion: "the binary's finder console: http:// line, session persistence and the clock, with a console-token now written in the state folder"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the door carries out stops first and makes 016's Origin, Host and peer checks as before (016 FR-002, FR-003, FR-012; 017 FR-006)"
      - "--listen keeps its default and the loopback hint its words"
    checks:
      development:
        - "cargo nextest run -p finder --locked --lib --test mcp --test cli -E 'test(/^pairing::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs and the field night of plan.md Outside the factory"
    writes:
      - crates/finder/src/main.rs
      - crates/finder/src/mcp/door.rs
      - crates/finder/src/pairing.rs
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/cli/pairing.rs
      - crates/finder/tests/mcp/main.rs
      - crates/finder/tests/mcp/pairing.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/main.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US1
        disposition: merge_order
        reason: "056 depends_on_landed 047, whose US1 last changed main.rs (d3f8bd0f)"
      - path: crates/finder/src/mcp/door.rs
        other_spec: 017-console-stop-confirm
        other_story: US1
        disposition: merge_order
        reason: "056 depends_on_landed 017, whose US1 last changed mcp/door.rs (5f547d07)"
      - path: crates/finder/tests/cli/main.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US1
        disposition: merge_order
        reason: "056 depends_on_landed 047, whose US1 last changed cli/main.rs (d3f8bd0f)"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "056 depends_on_landed 051, whose US3 last changed mcp/main.rs (9424a371)"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/console/pairing_docs.rs
        status: new
        disposition: add
        assertion: "README, every run of white space as one space: ### The console token after ### From a phone and before ## MCP clients, holding FR-013's eight sentences; the --listen row exactly FR-013's; ## MCP clients holds An MCP client on the console's own machine needs no token; another device's client sends the console token as `Authorization: Bearer <token>` (§ The console token). There is no console confirmation for MCP calls., no There is no token, and The imaging camera has sixteen tools of its own"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/console/pairing_docs.rs
        status: new
        disposition: add
        assertion: "GLOSSARY.md holds MCP client with FR-014's words and its _Avoid_ line, a blank line, then Console token with FR-014's words and _Avoid_: password, PIN, API key, node token"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: existing
        disposition: preserve
        assertion: "the README's ## API rows and ## MCP clients' sixteen tools of its own, as before"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every README sentence at f6d4f5fd but the --listen row and the MCP token sentence stays; every other glossary entry stays"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console -E 'test(/^pairing_docs::/) | test(/^imaging_run_docs::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - crates/finder/README.md
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/pairing_docs.rs
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "056 depends_on_landed 051, whose US3 last changed GLOSSARY.md (9424a371)"
      - path: crates/finder/README.md
        other_spec: 051-tools-page-focuses-either-camera
        other_story: US3
        disposition: merge_order
        reason: "056 depends_on_landed 051, whose US3 last changed README.md (9424a371)"
    open_questions: []
```
