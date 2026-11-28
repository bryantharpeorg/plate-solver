# Implementation Plan: The Console Opens to a Paired Phone

**Branch**: `056-console-opens-to-a-paired-phone` | **Date**: 2026-11-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/056-console-opens-to-a-paired-phone/spec.md`

## Summary

A new module, `crates/finder/src/pairing.rs`, holds the console token and one axum
middleware, the gate, that `finder::router` puts in front of every route. With a token,
every request but those to `/api/stop` and, while MCP is on, `/mcp` must name an allowed
host (012's `host_allowed`), a loopback peer's too; a loopback peer then passes. Any other
peer must present the token, as `Authorization: Bearer` (the scheme in any case) or as the
`console_token` cookie, compared without case, dashes or spaces, O read as 0 and I and L
as 1, through 013's `scope_link::tokens_match`; `POST /api/stop` always passes; wrong
tokens are counted per peer address, 10 in 60 s and then 429, at most 1,024 addresses.
Page paths answer an unpaired device with the console's own pairing prompt, a red-on-black
HTML form posting to `POST /pair`, which sets the cookie for 400 days; a wrong cookie is
cleared; a console without a token has no `/pair`. MCP's door asks the same, after its
stops. The binary reads the token from `console-token` in its state folder, or writes a
new one there through a temporary file (owner-only, 12 symbols of Crockford's base32 from
`/dev/urandom`), never prints it, and off loopback prints the address another device
opens. The React page, `dist`, `Cargo.toml` and `Cargo.lock` do not change. Four stories:
the gate; the prompt and its cookie; MCP, the binary's token and the browser test; the
words.

## Technical Context

**Language/Version**: Rust, the factory's toolchain; TypeScript for one Playwright test
(`crates/finder/web/e2e/`).

**Primary Dependencies**: `axum` 0.8.9 as the crate has it (`middleware::from_fn_with_state`,
the `ConnectInfo` extractor, which also reads `MockConnectInfo`), `scope-link`'s
`tokens_match`, the standard library's `/dev/urandom` read, and `finder::mcp`'s
`AddressSource` (`crates/finder/src/mcp/mod.rs:22`). No crate is added; no feature of a
crate changes.

**Storage**: `console-token` in the state folder, one line, mode 0600, written through
`console-token.tmp`. The wrong-token count lives in memory.

**Testing**: in-process router tests with `MockConnectInfo` peers (`console/` and
`mcp/` test binaries), one with a simulated live loop, the built binary on loopback
(`cli/`), unit tests in `pairing.rs`, and one Playwright test in the default project
against the release binary on 127.0.0.1:8097.

**Target Platform**: the console on the scope node's computer (Linux on AArch64), a desk
console on Linux or macOS; `/dev/urandom` exists on all of `deny.toml`'s three targets.

**Constraints**: loopback answers byte-for-byte as before this spec at the console's own host
(FR-002); Stop answers everyone (014, 017); no answer's body holds the token; the gates
make no network request and bind nothing but loopback.

**Scale/Scope**: one module of about 560 lines, three lines in `api.rs`, three in the
door, ten in `main.rs`, five test files and the rig, a README section, passages of three
other pages and two glossary entries.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing of the solver, detection or numerics changes. The gate's behaviour is grounded in its cited sources: RFC 6265 (cookies, `HttpOnly`, `SameSite`), RFC 6750 and RFC 7235 (`Authorization: Bearer`, the scheme without case, `WWW-Authenticate`), Crockford's base32, 012's same-origin and host rules, 013's token comparison, 014's and 017's Stop. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS`, `ergane.yaml` or `playwright.config.ts` is edited. The only edits to existing test files are the `mod` lines of the `console`, `mcp` and `cli` test binaries and the rig's module at the end of `crates/finder/tests/support/mod.rs` (SC-002); a file of its own there fails 027's layout test (`crates/finder/tests/layout/main.rs:126`, run). Every existing test passed unchanged at each story's tip (run, § What was run). |
| III. Tests first, independent expectations | The oracles are the answers a loopback peer and the plain console give, the scenarios' words, status codes and headers, the alphabet and the file's mode, the live loop's own report and preview, and DESIGN.md's night tokens. No test reads `WRONG_TOKENS`, `WRONG_WINDOW_S` or `COOKIE_MAX_AGE_S`. Time comes from a clock the test sets. Every story's new tests failed on the story before it (run). |
| IV. Read the critical paths twice | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/`, `crates/math-core`, `crates/pattern-database` or `crates/database-generation`, no `build.rs` or `.cargo/`. The gate is a security path: its reason is spec.md's input, its diff is one module and three call sites, and every refusal and every pass has a test that fails without it (§ What was run, the mutations). |
| V. Proven open source first, with honest licensing | Looked for, in local sources only (no download), in `~/.cargo/registry` and the repository's `reference-solutions/`: **`tower-http`** (its `ValidateRequestHeaderLayer::bearer` and CORS): not in the registry or `Cargo.lock`, and a bearer layer alone covers neither a cookie, a loopback exemption, Stop, a prompt nor a limit; **`axum-extra`'s cookie jar** and **`cookie`** 0.18.1 (in the registry, not in `Cargo.lock`): a cookie is one `Set-Cookie` line and one `split(';')`, not worth a dependency change; **`tower_governor` / `governor`** (rate limiting): not in the registry; a 60 s window per address is 20 lines, after 016's own `MoveLimit` (`crates/finder/src/mcp/limit.rs`); **axum's own `form` feature** (`axum::Form`, its `serde_urlencoded` 0.7.1 already in `Cargo.lock` through `query`): measured against the hand-written `form_field`, owner decision 10; **`getrandom`** 0.4.3 (in `Cargo.lock` through `rmcp`, MIT OR Apache-2.0): owner decision 2 (b), turned down because a direct dependency is a `Cargo.toml` and `Cargo.lock` change for 12 bytes the standard library reads from `/dev/urandom` on every target, as `crates/finder/src/simulate.rs:64` already does; **`rustls`** 0.23.46 (in the registry, not in `Cargo.lock`) and a certificate generator (none found): owner decision 5 (b); a **QR encoder**: none in the registry or the page's `node_modules` (whose one cookie package, `tough-cookie`, is jsdom's client jar), owner decision 3 (c). A search of `reference-solutions/` for `Set-Cookie`, `WWW-Authenticate` and bearer tokens found nothing; it holds cedar only, so Home Assistant, OctoPrint, Jupyter, INDI Web Manager and Ekos Live have no local sources (from memory, Jupyter's start-time token traded for a cookie is this design's shape). So the gate is written fresh, on the console's own host rule, 013's comparison and 016's door. |

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
crates/finder/src/pairing.rs                    # new (US1, the gate); US2 the prompt and cookie; US3 the token file, MCP's check and the start line
crates/finder/src/api.rs                        # US1: the `pairing` field, `router` split; US2: `pairing::routes(&state)`
crates/finder/src/lib.rs                        # US1: `pub mod pairing;` and a doc paragraph; US2: the `/pair` line
crates/finder/src/mcp/door.rs                   # US3: three lines after the peer check
crates/finder/src/main.rs                       # US3: the token file and the start line
crates/finder/tests/support/mod.rs              # US1: `pub mod pairing { ... }` at its end, the rig
crates/finder/tests/console/pairing.rs          # US1, new
crates/finder/tests/console/pairing_prompt.rs   # US2, new
crates/finder/tests/mcp/pairing.rs              # US3, new
crates/finder/tests/cli/pairing.rs              # US3, new
crates/finder/web/e2e/pairing.spec.ts           # US3, new (default project; no config change)
crates/finder/tests/console/pairing_docs.rs     # US4, new
crates/finder/README.md, GLOSSARY.md            # US4
README.md, docs/safety-and-security.md, docs/guide/console.md   # US4
```

**The other drafts' files.** 055 writes `crates/finder/src/api.rs` (a field before
`move_limit`, one `merge` before `routes.with_state(state)`) and `crates/finder/src/lib.rs`
(`pub mod nudge;`), and adds `mod nudge;` to the test binaries' `main.rs` files; 052 to
055 all write `crates/finder/README.md`, `GLOSSARY.md` and the test binaries' `main.rs`
files; 057 and 058, drafted the same day, write `api.rs` or `lib.rs`, the README, the
glossary and `playwright.config.ts`. This spec keeps out of their lines: its `AppState`
field follows `allowed_hosts`, its router change is at the top of `router`, its module
line is after `pub mod imaging;`, its README section sits inside `### From a phone`, and
every `mod` line goes in its alphabetical place. 052, 053, 054 and 055 land in that
order, and the seat re-anchors this trio on the `main` that holds them, and 057 or 058
if either lands first, before any flip (§ Prerequisites).

## Interfaces

### `crates/finder/src/pairing.rs` (US1 new; US2 and US3 add)

Module doc: what spec.md's FR-001 to FR-005 say, in the module's own words, and (US3)
that the binary keeps the token in `console-token`.

**US1:**

- `pub const COOKIE: &str = "console_token";`
- `pub const WRONG_TOKENS: usize = 10;` and `pub const WRONG_WINDOW_S: f64 = 60.0;`
- `const MAX_PEERS: usize = 1024;`
- `pub struct ConsoleToken { normal: String }`, `#[derive(Clone)]` only, never `Debug` or
  `Serialize`. `fn normalize(text: &str) -> String` drops `-` and white space,
  upper-cases ASCII, and maps `O` to `0` and `I` and `L` to `1`. `pub fn new(text: &str)
  -> Result<ConsoleToken, InvalidToken>` accepts a normalized 8 to 64 ASCII letters and
  digits. `fn matches(&self, supplied: &str) -> bool` is
  `scope_link::tokens_match(normalize(supplied).as_bytes(), self.normal.as_bytes())`.
- `pub struct InvalidToken;` (`Debug`, `Display` = `a console token is 8 to 64 letters
  and digits; dashes and spaces are ignored`, `std::error::Error`).
- `pub(crate) struct Pairing { token: ConsoleToken, wrong: Mutex<HashMap<IpAddr,
  VecDeque<f64>>> }` (a `std::sync::Mutex`, never held across an await) and `fn
  check(&self, peer: IpAddr, supplied: &str, now: f64) -> Check`: first, in every
  address, keep only the times `at` with `at <= now && now - at < WRONG_WINDOW_S`, and
  drop every emptied address; then `Limited` if the address holds `WRONG_TOKENS` times;
  `Admitted` if the token matches; `Limited` if the address is new and `MAX_PEERS` are
  held; else record `now` and `Wrong`.
- `impl AppState { pub fn with_console_token(mut self, token: ConsoleToken) -> Self }`,
  setting `api.rs`'s new `pub(crate) pairing: Option<Arc<Pairing>>`.
- `fn loopback(ip: IpAddr) -> bool`: IPv4 `is_loopback`, IPv6 `is_loopback` or an
  IPv4-mapped loopback.
- `async fn peer(parts: &mut Parts) -> Option<IpAddr>`:
  `ConnectInfo::<SocketAddr>::from_request_parts(parts, &())`, the extractor, because
  `MockConnectInfo` stores itself, not a `ConnectInfo`, in the extensions (plan trap 3).
- `fn presented(headers: &HeaderMap) -> Option<(String, bool)>`: an `Authorization`
  header gives, when `split_once(' ')` finds a scheme equal to `bearer` ignoring ASCII
  case, the rest, and otherwise the empty string (so `Basic` counts as wrong), with
  `false`; else the first `console_token=` pair of any `Cookie` header split on `;` and
  trimmed, with `true` (from the cookie).
- `const UNPAIRED`, `WRONG`, `LIMITED: (&str, &str)`: the prompt's words (US2) and the
  API's words: `("Pair this device: enter the console token.", "this device is not
  paired: open the console's page and enter the console token")`, `("That is not the
  console token.", "that is not the console token")`, `("Too many wrong console tokens
  from this device: wait a minute and try again.", "too many wrong console tokens from
  this device: wait a minute")`.
- `pub(crate) async fn gate(State(state): State<AppState>, request: Request, next: Next)
  -> Response`: no `pairing` → `next`; split the request; the path `/api/stop` (any
  method), or `/mcp` when `state.mcp.enabled` (its door asks, US3) → `next`; with MCP off
  `/mcp` is a page path and goes on as `/` does; `!api::host_allowed` → 403 `ApiError`
  with `open the console at an IP address, localhost or an allowed host`; a loopback peer
  → `next`; no token → 401; `Admitted` → `next`; `Wrong` → 401; `Limited` → 429; each
  refusal an `ApiError` with the API's words. A request with no peer counts under
  `0.0.0.0`.
- Unit test of `new` and `matches`: `abcd-efgh-jkmn` matches `ABCDEFGHJKMN` and
  `' abcd efgh jkmn '`, not `ABCDEFGHJKMP` or `ABCDEFGHJKM`; `10ab-cdef-gh0j` matches
  `lOAB-CDEF-GHoJ` and `IOAB CDEF GHOJ`; `short`, `not/a/token` and 65 letters are
  refused.

**US2 adds:**

- `pub const COOKIE_MAX_AGE_S: u64 = 400 * 24 * 60 * 60;` and `const FORM_LIMIT: usize
  = 1024;`
- In `gate`: after the loopback pass, the path `/pair` → `next` too; a 401 or 429 refusal
  on a path not under `/api/` is `(status, prompt_html(words))`; the 403 for a host stays
  the JSON error on every path (US1-S1 pins `/`); a `Wrong` whose token came from
  the cookie adds `Set-Cookie: console_token=; Path=/; Max-Age=0; HttpOnly;
  SameSite=Strict`.
- `pub(crate) fn routes(state: &AppState) -> Router<AppState>`: with `state.pairing`
  `Some`, `Router::new().route("/pair", get(prompt).post(pair))`; with `None`,
  `Router::new()`, so a console without a token answers `/pair` as before this spec (FR-006).
  `prompt` answers 200 with the prompt and UNPAIRED's words. `pair` runs
  `api::check_origin` first (its 403 as it is); reads at most `FORM_LIMIT` bytes (a bare
  413 past it); takes `form_field(body, "token")` (`&`-split pairs, `+` as space, `%XX`
  decoded; the `Content-Type` is not read) or the empty string, and `check`s it under the
  peer's address: `Admitted` → 303, `Location: /`, `Set-Cookie: console_token=<normal>;
  Path=/; Max-Age={COOKIE_MAX_AGE_S}; HttpOnly; SameSite=Strict`; `Wrong` → 401 prompt;
  `Limited` → 429 prompt.
- `fn prompt_html(message: &str) -> Html<String>`, exactly this page with `{message}`
  in place (FR-010; US2-S4 reads its text). Eleven `#` colours, all night tokens:

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
label { display: block; margin: 1rem 0 0.25rem; font-size: 0.875rem; font-weight: 600; color: #d40000; }
input, button { box-sizing: border-box; width: 100%; min-height: 44px; border: 1px solid #e00000; border-radius: 0; font: inherit; color: #e00000; caret-color: #e00000; }
input { padding: 0 12px; background: transparent; letter-spacing: 0.1em; text-transform: uppercase; }
button { margin-top: 1rem; padding: 0 16px; background: #5a0000; font-weight: 560; }
input:focus, button:focus { outline: 2px solid #e00000; outline-offset: 2px; }
::selection { background: #5a0000; color: #e00000; }
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
<p>The console token is in the file console-token in the console's state folder. This browser keeps it until the token changes.</p>
</main>
</body>
</html>
```

  (In Rust, a raw string `r##"..."##` through `format!`, the CSS braces doubled.)
- Unit test of `form_field`: `token=AB+CD%2DEF` gives `AB CD-EF`, `x=1&token=Q` gives
  `Q`, `x=1` none.

**US3 adds:**

- `pub const TOKEN_FILE: &str = "console-token";` and `const ALPHABET: &[u8; 32] =
  b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";`
- `pub fn generate() -> io::Result<ConsoleToken>`: 12 bytes `read_exact` from
  `/dev/urandom`, each `b & 31` into `ALPHABET` (256 is a multiple of 32, so uniform).
- `pub fn grouped(&self) -> String`: the normal form in groups of four joined by `-`.
- `pub fn load_or_create(path: &Path) -> Result<ConsoleToken, String>`: read the file and
  `new` it, the error `"{path}: {InvalidToken}"`; on unix, when its mode has any of
  `0o077`, `eprintln!` `"{path}: other users can read the console token; chmod 600 the
  file"` and keep it; if it does not exist, `generate`, and write `grouped()` and a
  newline through `write_owner_only`; other read errors `cannot read {path}: {error}`.
  Never the contents in an error.
- `fn write_owner_only(path: &Path, text: &str) -> io::Result<()>`, as
  `crates/finder/src/session.rs:2060` to `crates/finder/src/session.rs:2100` writes the session: create the folder;
  `path.with_added_extension("tmp")`, removing any left there (`NotFound` is fine);
  `OpenOptions` `write`, `create_new` and, on unix, `mode(0o600)` on it; `write_all`,
  `sync_all`, then `fs::rename` to `path`.
- `pub fn listening_hint(bound: SocketAddr, addresses: &dyn crate::mcp::AddressSource,
  token_path: &Path) -> String`: a loopback `bound.ip()` → `(to open it from a phone,
  run with --listen 0.0.0.0:8080)`, `addresses` not asked; else the address is
  `bound.ip()`, or for an unspecified one the first of `addresses.addresses()` that is
  not loopback; with one, `other devices open http://{SocketAddr::new(ip,
  bound.port())} and pair with the console token in {path}`; with none, `other devices
  pair with the console token in {path}`.
- `pub(crate) async fn admits_mcp(parts: &mut Parts, state: &AppState) ->
  Option<Response>`: `None` with no `pairing`, for a loopback peer or the right token
  (from `presented`, so a bearer header or the cookie); else `Some` of `(status,
  [(WWW-Authenticate, "Bearer realm=\"finder console\"")], Json({"jsonrpc": "2.0",
  "error": {"code": -32001, "message": M}}))` with 401 and `MCP from another device
  needs the console token as a bearer token` (no token), 401 and WRONG's API words, or
  429 and LIMITED's API words. (Returning `Option`, not `Result`: clippy's
  `result_large_err` refuses a `Response` error.)
- Unit tests: `grouped` gives `ABCD-EFGH-JKMN` for `abcdefghjkmn` and
  `MY0W-NC0N-S01E-T0KE-N` for `my-own-console-token`; `generate` gives 12 symbols of the
  alphabet, and two generations differ; `listening_hint` as US3-S5, with a test
  `AddressSource` that returns a fixed list.

### `crates/finder/src/api.rs` (US1; US2 one call)

- `AppState` gains `pub(crate) pairing: Option<Arc<crate::pairing::Pairing>>` after
  `allowed_hosts` (`crates/finder/src/api.rs:115`), `None` in `shared` after
  `allowed_hosts: Arc::new(Vec::new()),` (`crates/finder/src/api.rs:159`), and its `mcp`
  field (`crates/finder/src/api.rs:116`) becomes `pub(crate) mcp`, so the gate reads
  `state.mcp.enabled`.
- `router` (`crates/finder/src/api.rs:621`) becomes:

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
  `crate::pairing::routes(&state)`. `Router::layer` after every `merge` and the fallback
  puts the gate before every route, the page and `/mcp`'s door among them.

### `crates/finder/src/lib.rs` (US1; US2 one line)

US1: after `pub mod imaging;` (`crates/finder/src/lib.rs:116`), a blank line,
`/// The console token's gate for devices other than the console's own machine (056).`
and `pub mod pairing;`; after the paragraph ending `[`AppState::with_allowed_hosts`].`,
`//!` and: "With a console token ([`AppState::with_console_token`]), every request must
name an allowed host, and one from a peer that is not loopback must also present the
token, as the `console_token` cookie or a bearer header, before any route but
`POST /api/stop` answers ([`pairing`])." US2 puts `` `/pair` and `` before
`` `POST /api/stop` `` there, and after the route list's last line `//! * GET /pair: the
pairing prompt; POST /pair: a form with the console token, answered 303 with its cookie
(056; only with a console token).`

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
finder::pairing::ConsoleToken::load_or_create(&token_path).unwrap_or_else(|error| {
eprintln!("{error}"); std::process::exit(1) });`; `.with_console_token(token)` after
`.with_allowed_hosts(args.allow_host)` (`crates/finder/src/main.rs:199`); and the loopback
hint's `if` (`crates/finder/src/main.rs:277` to `:279`) replaced by `eprintln!("{}",
finder::pairing::listening_hint(bound_addr, McpConfig::default().address_source.as_ref(),
&token_path));`. `--listen`'s default and doc stay. A bad file's error is printed as it
is, the file's path and the words, and the start exits 1 before it binds. Returned from
`main` with `?`, Rust would print it in Debug form, `Error: "<path>: …"`, escaping a
path's quotes, backslashes and non-ASCII (US3-S4 reads the whole line).

### The rig (`crates/finder/tests/support/mod.rs`, US1)

At the end of the file, after `Scratch`'s `Drop`: a doc comment saying what the rig is
and that it lives here because 027's layout test pins the folder's files, then
`pub mod pairing { ... }` holding `TOKEN`, `T` (the scenarios' start time,
`1_768_528_800.0`), `PHONE`, `OTHER_PHONE`, `HOST`, `ORIGIN`, `COOKIE`, `UNPAIRED`,
`WRONG`, `LIMITED`, `BAD_HOST`, `READS`, `StepClock` (a `Clock` over a `Mutex<f64>`, with
`set(&self, now: f64)` for US1-S4 and US1-S7), `clock()` (a `StepClock` at `T`),
`gated(clock)` (`super::mcp::state()` with the clock and
`ConsoleToken::new(TOKEN).unwrap()`), `from(state, peer)`, `get(path, host, header)`,
`send`, `json_of` and `answer` (status, content type, body). The binaries already include
`support` with `#[allow(dead_code, unused_imports)]`, so items one binary does not use are
not dead code to clippy.

### The README (`crates/finder/README.md`, US4)

The `--listen` row (`crates/finder/README.md:34`) and the `--allow-host` row
(`crates/finder/README.md:43`) become:

```text
| `--listen ADDR` | `127.0.0.1:8080` | Address to serve on. `0.0.0.0:8080` lets other devices connect; they must present the console token (§ The console token). |
| `--allow-host NAME` | none | DNS name the console answers, in addition to IP literals and `localhost`; any other name gets only `POST /api/stop`. Repeat for multiple names. |
```

After the location paragraph (`crates/finder/README.md:61` to `:63`), a blank line and:

```markdown
### The console token

Every device other than the console's own machine must present the console token
before the console answers it. At its first start the console writes a new token,
twelve letters and digits in three groups such as `ABCD-EFGH-JKMN`, to `console-token`
in its state folder, readable by its owner only; it never prints the token. Read it
over your own ssh, for example with `cat ~/.local/state/finder/console-token`, or in
the folder the console's `--state` names. Or write your own token there, 8 to 64
letters and digits you can type from memory at the eyepiece, and start the console
again.

Pair the phone once, at home: start the console with `--listen 0.0.0.0:8080` and
`--allow-host` with its computer's `.local` name, open that name on the phone, such as
`http://<the computer's name>.local:8080`, and type the token into the prompt. Case,
dashes and spaces do not matter, and O reads as 0, I and L as 1. The browser keeps it
in a cookie for 400 days, for that name only, so a new address or name asks again. If
the phone cannot find the name, open the address the console's start line prints
instead. Then turn Night on once; the page keeps it for the same name. Every page,
live stream and picture then works as it does on the console's own machine. Another
device's MCP client, or curl, sends the token as `Authorization: Bearer <token>`.

- Clients on the console's own machine are never asked for the token, an ssh tunnel to
  127.0.0.1:8080 among them.
- Every request but `POST /api/stop` must open the console at an IP address,
  `localhost` or a name given with `--allow-host`, from the console's own machine too.
- `POST /api/stop` and MCP's `stop` and `stop_all` stop the mount from any device and
  any web page, paired or not.
- A device that sends 10 wrong tokens within 60 s is answered 429 until the oldest of
  them is a minute old.
- To change the token, stop the console, delete `console-token` or write your own in
  its place, and start the console again; every paired browser then asks again.
- A browser sends the cookie to every service at the console's address, whatever its
  port, so run one console per address.
- The console listens on every network its computer joins, and it speaks plain HTTP,
  so the token crosses the air unencrypted on every request. Use it on a private
  network you trust, such as a phone's own hotspot, never on a public one.
```

In `## MCP clients`, `There is no token or console confirmation for MCP calls. Every
moving tool runs` (`crates/finder/README.md:80`) becomes:

```text
An MCP client on the console's own machine needs no token; another device's client
sends the console token as `Authorization: Bearer <token>` (§ The console token).
There is no console confirmation for MCP calls. Every moving tool runs
```

### The other pages (US4)

`README.md:133` to `README.md:137` becomes (the sentence before it, the link targets and the
paragraph after unchanged):

```markdown
Every moving tool runs the page's guards, the page says when an MCP client is moving the
telescope, and the page's Stop ends any motion. The console listens on localhost by default,
and MCP calls from its own machine carry no token. With `--listen 0.0.0.0:8080`, every other
device must present the console token, a phone once through the console's prompt and an MCP
client as a bearer token, though any device can still stop the mount. See
[the console manual](docs/guide/console.md) and
[safety and security](docs/safety-and-security.md#network-exposure).
```

and `README.md:256` becomes two lines: `- The console asks other devices for a token but
speaks plain HTTP, and no test pairs a phone` / `  over a real Wi-Fi. It listens only on
localhost by default.`

`docs/safety-and-security.md:58` to `:69` (the bold line and the five bullets) become:

```markdown
**The console asks every other device for a token, over plain HTTP.**

- By default it listens on `127.0.0.1:8080`, so only the same computer can reach it.
- With `--listen 0.0.0.0:8080`, it listens on every network its computer joins. Every device
  other than that computer must then present the console token, kept by a phone's browser as a
  cookie or sent by an MCP client as a bearer header, before the console answers it; a device
  that sends 10 wrong tokens within 60 s is answered 429. The crate reference's
  [console token](../crates/finder/README.md#the-console-token) section says where it is.
- **The token crosses the air in the clear** on every request, so anyone who can listen to the
  network can copy it. Only do this on a network you trust, such as a phone's own hotspot.
- **Any device, and any web page open in a browser on that network, can stop the mount.**
  `POST /api/stop` needs no token, no `Origin` and no allowed host.
- **Browser writes are protected against other websites.** A write must come from the console's
  own origin, and the `Host` header must be an IP address, `localhost` or a name you allowed with
  `--allow-host`. That blocks DNS rebinding.
- **Every request but Stop must name an allowed host,** from the console's own computer too, so
  a page on a rebound name cannot read the console either.
- **Requests with no `Origin` header are allowed.** A script such as `curl` is not a browser, so
  it is accepted.
- **`/mcp` takes the same token from another device.** It also requires the caller to be on the
  same machine or a private network address (loopback, RFC 1918, link-local, or IPv6 ULA).
  `--no-mcp` turns it off.
```

In `docs/guide/console.md`, the `--allow-host` row (`docs/guide/console.md:55`) becomes:

```text
| `--allow-host NAME` | none | A DNS name allowed in the `Host` header, besides IP addresses and `localhost`; any other name gets only Stop. Repeatable. |
```

and the **Security.** paragraph (`docs/guide/console.md:96` to `docs/guide/console.md:97`)
becomes two paragraphs:

```markdown
**The phone asks for the console token once.** Read it from `console-token` in the console's
state folder over your own ssh, or write your own token there; the crate reference's
[console token](../../crates/finder/README.md#the-console-token) section says how. Pair by the
name you will use at the telescope, since the browser keeps the pairing for that name only.

**Security.** Every other device must present the console token, but the console speaks plain
HTTP and any device can stop the mount. See
[network exposure](../safety-and-security.md#network-exposure).
```

Three more passages change: `docs/guide/console.md:105` to `docs/guide/console.md:106`,
`docs/guide/console.md:161` to `docs/guide/console.md:162`, and
`docs/guide/console.md:168` to `docs/guide/console.md:169` become, in that order:

```markdown
**If you reach the console by name** (for example a `.local` name), add that name with
`--allow-host`. Otherwise the console answers that name only with Stop.
```

```markdown
The console serves MCP (Streamable HTTP) at `/mcp` on the same port as the page. It is
stateless, and on the console's own machine it takes no token. Setup's **MCP** section shows
the address to copy.
```

```markdown
- **From another device,** use the console's network address. `/mcp` answers only callers on
  the same machine or a private network address, and needs `--listen 0.0.0.0:8080` and the
  console token as `Authorization: Bearer <token>`.
```

### The glossary (`GLOSSARY.md`, US4)

**MCP client** (`GLOSSARY.md:265` to `GLOSSARY.md:267`) takes FR-014's words, its
`_Avoid_` line unchanged, then a blank line and **Console token** with FR-014's words and
`_Avoid_: password, PIN, API key, node token`.

### The browser test (`crates/finder/web/e2e/pairing.spec.ts`, US3)

`test.use({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 })`; one test
named `US3-S6: ...`; `page.goto("/pair")`, `getByRole("heading", ...)`,
`getByLabel("Console token")`, `getByRole("button", { name: "Pair" })`, one
`page.evaluate` collecting the six computed colours of `document.documentElement` and
`body, body *` (skipping `rgba(0, 0, 0, 0)`), each matched against
`/^rgba?\((\d+), 0, 0(?:, [\d.]+)?\)$/`, `toHaveCSS` for `html`'s background and the
focused field's outline, `boundingBox()` heights, then `fill` and `click` and
`getByRole("status")`. Its file name is not in the default project's `testIgnore`
(`crates/finder/web/playwright.config.ts:36` to `crates/finder/web/playwright.config.ts:47`), so it runs there with no
config change, against 127.0.0.1:8097, whose binary has a token from US3 on.

## Opening the console from a phone

What the owner does, written for the README and for § Outside the factory:

1. Start the console with --listen 0.0.0.0:8080 and --allow-host with its computer's
   `.local` name; its start line names the address another device opens and the token's
   file.
2. Read the token from that file over your own ssh, or write your own there, 8 to 64
   letters and digits, and start the console again.
3. At home, on the same Wi-Fi, open http://<the computer's name>.local:8080 on the phone
   (or the address the start line prints), type the token into the prompt, press Pair,
   then turn Night on. The page opens and stays paired, at that name, for 400 days.
4. The tunnel and Claude keep working as before.

## What was run and what was read

The drafting round and review round 1's fold ran on the probe branch `probe/056`, cut
from f6d4f5fd; probe round 1 and its fold on `probe/056-r1`, cut from 745dc3a8 (f6d4f5fd
and this trio). All on the factory's toolchain with `--locked`, npm from its cache with
`npm ci --offline`, the browsers from the factory's Playwright cache. Logs are outside
the repository.

**The drafting round.** ef9009de (US1); fcc4bd2d (US2) with its fold e5f3c146;
891e1805 (US3 and US4 together), split by a0447d0c (PROBE ONLY) and 9061c9d8 (US4), with
the three folds 4e068469, 7836bd15 and b2513833 (tests only). Its gates ran once, at the
tip, so the review found that US2's browser test could not pass on US2's own diff (no
token in the binary until US3; run: exit 1, the page `{"error":"this console asks for no
token"}`).

**Review round 1's fold.** bbab2777 (PROBE ONLY: US2, US3 and US4 backed out to
ef9009de's tree), then each story rebuilt on the one before: 1608bc31 (US1) with its two
folds 4078f4ea (the live stream's scenario to US2) and 250868fc (the rig into
`support/mod.rs`), c0fa8609 (US2), c4902bf9 (US3) and 1aba3c3c (US4). Every gate ran at
each story's own tip: fmt, clippy with `-D warnings`, both `cargo doc` runs, nextest, the
doc tests, typecheck, Prettier, Vitest, the build, the `dist` check and `CI=1 npm run
e2e` after `npm ci --offline` each exited 0 at 250868fc, c0fa8609, c4902bf9 and
1aba3c3c (nextest 1,533, 1,539, 1,546 and 1,549 tests; Vitest 607 at each; the browser
tests 34, 34, 35 and 35). At 4078f4ea, with the rig in a file of its own under
`support/`, 027's layout test failed, so the rig moved.

**Probe round 1, from this text alone, and its fold.** A builder who read only the trio
built the four stories on `probe/056-r1`, each a tests-first commit and the story:
212d637e and 8d236be3 (US1), 27fb9c02 and d4a631e5 (US2), 17e1e677 and 481092d9 (US3),
4e395e46 and 6fe4f173 (US4). It found 0 blocking and 16 clarity defects (056-R1 to
056-R16, all folded here); every story's new tests failed on the story before it, and
every gate passed at each story's tip (nextest 1,533, 1,539, 1,546 and 1,549; Vitest 607;
browser tests 34, 34, 35 and 35). The fold's code went on that branch on top of 6fe4f173,
one commit per story: 5ca389bf (US1: the gate passes `/mcp` only while MCP is on, the
`mcp` field `pub(crate)`; the new steps of US1-S1, US1-S2, US1-S3 and US1-S6), 0346d61d
(US2: US2-S1's step) and e4335286 (US3: `main` prints a bad file's line and exits 1;
US3-S1's step and US3-S4's whole line). Each new step passed on the code and failed under
its mutation, one at a time: `mcp-passes-when-off` (US1-S2's `/mcp`, 200 for 401),
`ipv4-only-127.0.0.1` (US1-S1's 127.0.0.2), `emptied-addresses-kept` (US1-S6 at T + 60 s,
429 for 401), `cookie-read-first` (US1-S3), `focus-routes-ungated` (US1-S2's focus route,
the gate layered before 051's merge), `pair-before-host-check` (US2-S1, 200 for 403),
`token-before-016-checks` (US3-S1's 203.0.113.7, 401 for 403) and
`bad-file-returned-from-main` (US3-S4: the line read `Error: "…"`). Comparing with `==` in
place of `tokens_match` passes every test, so it is read from the diff. At e4335286 every
command of `lint` and `test` exited 0, `CI=1 npm run e2e` among them, and so did
cargo-deny (offline) and cargo-machete: nextest 1,549, Vitest 607, browser tests 35.

- **Every story's new tests failed on the story before** (run): US1's do not build on
  f6d4f5fd (no `finder::pairing`); US2's five fail on US1's tip; US3's MCP tests fail on
  US2's tip (the phone's call with no token answered 200), its CLI tests too (no
  `console-token` written, no warning), and its browser test finds no prompt at `/pair`
  there (the page's fallback); US4's three fail on US3's tip.
- **Mutations, one at a time, each failing its scenario's test** (run): 25, each exit 100
  with its scenario's test failing. US1: `bearer-case-sensitive`, `no-crockford-lookalikes`,
  `loopback-skips-host-check`, `no-host-check`, `stop-gated`, `no-peer-is-loopback`,
  `ipv4-loopback-only`, `one-count-for-all`, `no-address-cap`, `later-times-kept`. US2:
  `pair-without-a-token`, `form-limit-doubled`, `label-third-ink`, `cookie-kept`,
  `pair-any-origin`, `stream-refused`. US3: `mcp-header-only`, `stale-temporary-kept`,
  `mode-0644`, `no-shared-file-warning`, `hint-ignores-the-bound-address`,
  `token-not-asked-by-mcp` (with `admits_mcp` still named: dropping it fails the build
  under `-D warnings`). US4: `safety-page-says-no-login`, `glossary-overloads-pairing`,
  `readme-drops-one-console-per-address`.
- **Loopback is unchanged at the console's own host (US1-S1).** With a token,
  127.0.0.1, ::1 and ::ffff:127.0.0.1 got the plain console's status, content type and
  bytes for the six reads with `Host` localhost:8080 and 127.0.0.1:8080, and 403 with a
  rebound name or none; Stop 200 with a rebound name; `PUT` 200.
- **Owner decision 1 (e), the Host check for loopback too.** Before it (the draft, and
  the mutation `loopback-skips-host-check`), 127.0.0.1 with `Host` rebound.example:8080
  read `/api/session` and `/api/mcp` 200; with it, 403. With the binary's token, every
  existing test and all three browser projects passed at US3's and US4's tips. It adds
  no item to the gate: the draft's Host check moved above the loopback pass and its
  words lost "another device must"; US1-S1 gained the rebound and missing-`Host` reads
  from each loopback peer.
- **Owner decision 1 (f), Stop refusing a foreign `Origin`** (an experiment, not kept):
  with 012's `check_origin` before Stop's pass in the gate, the phone's `POST /api/stop`
  with `Origin` http://evil.example, `Host` rebound.example:8080 and a `text/plain`
  body went from 200 to 403, and so did loopback's with that `Origin`; with no `Origin`,
  or the page's own, it stayed 200, and US1's tests and 014's foreign-origin Stop (no
  token) passed. With the same check on the route of every console, 4 of the motion
  binary's 66 stop tests failed: `crates/finder/tests/motion/goto/sim.rs:188`,
  `crates/finder/tests/motion/tracking/routes.rs:209`,
  `crates/finder/tests/motion/tracking/routes.rs:408` and
  `crates/finder/tests/motion/goto/second_sync.rs:131`.
- **The cap (US1-S6).** 1,024 addresses with one wrong token each answered 401; the
  1,025th's wrong token 429 and its right token 200 (review round 1, with 1,100
  addresses × 10 in one window: 10,240 guesses counted before new addresses were
  refused).
- **The live stream with the cookie (US2-S5).** On a simulated live loop, the cookie the
  prompt set opened `/api/live` (200, `text/event-stream`), whose first chunk held
  `event: live` and the loop's own first report, and the preview that report names
  (200, `image/jpeg`, the loop's held JPEG), equal to loopback's; with no token, both
  401. On the gated console of US1 and US2-S2, which has no live loop and no imaging,
  `/api/live` and both previews answer 404 to everyone (review round 1).
- **Owner decision 10, the form.** With axum's `form` feature and `axum::Form` in place of
  `form_field` at US2's tip (an experiment, not kept), `Cargo.lock` was unchanged under
  `--locked` and US2's tests passed with `form_field`'s unit test removed. Both readers
  gave 303 to `test-CONSOLE+token` and `TEST%2DCONSOLE%2DTOKEN` and the prompt (401) to
  `%zz` and to a body with no `token` field; a `text/plain` body with the right token got
  303 from `form_field` and 415 in plain text from `axum::Form`. US2's diff: 23,397 B, and
  23,413 B with `axum::Form`.
- **`MockConnectInfo` is not `ConnectInfo`.** The first gate read
  `extensions().get::<ConnectInfo<SocketAddr>>()` and saw no peer in every test, so every
  loopback test answered 401; the extractor reads both (plan trap 3).

Read, not run: the phone on a real Wi-Fi, whether its browser resolves the node's
`.local` name on a phone's own hotspot, and the scope node's service and firewall
(§ Outside the factory); that a browser's `EventSource` and `<img>` send a same-origin
cookie (the HTML standard, from memory); `SameSite=Strict` and `Lax` on links from other
apps (RFC 6265bis, from memory); that cookies are not isolated by port (RFC 6265 § 8.5,
from memory); WSL2 NAT and portproxy peers (Edge Cases, from the README's own section);
a real phone browser's handling of a self-signed certificate (owner decision 5 (b)) and
of a cookie's 400 days (owner decision 4); RFC 6750 and RFC 7235, cited from memory. The
binary was started only on 127.0.0.1; the line it prints off loopback is US3-S5's unit
test.

## Read from the diff, not from a test

- No answer's body holds the token; `ConsoleToken` has no `Debug`, `Display` or
  `Serialize`.
- `Pairing`'s mutex is never held across an await.
- `ConsoleToken::matches` compares through `scope_link::tokens_match`, never `==`.
- `main.rs` loads the token before it binds, so a bad file stops the start before the
  `finder console:` line.
- `write_owner_only` writes a temporary file and renames it; no test interrupts a write,
  so the rename is read here (US3-S3 shows a left temporary file removed).
- `main.rs` passes `McpConfig::default().address_source`, the outward-address source
  Setup uses (`crates/finder/src/mcp/mod.rs:57`), to `listening_hint`.

## Shapes that existing tests pin

- **016's door.** `crates/finder/tests/mcp/endpoint.rs:202` builds `app(state(), peer)`
  with no console token and expects LAN peers 200 and public peers 403: unchanged,
  since a console without a token asks nothing (FR-006) (run).
- **012's same-origin rule.** `crates/finder/tests/console/session/writes.rs` sends
  writes with and without `Origin` from routers with no peer and no token: unchanged
  (run).
- **014's Stop from a foreign origin.** `crates/finder/tests/motion/goto/sim.rs:188`
  sends `POST /api/stop` with `Origin: http://foreign.invalid` and expects 200 (014's
  US6-S11, FR-020): unchanged, since the gate passes Stop first (run).
- **Routers with no peer.** Of the 66 test files that name `router(`, 4 mock a peer;
  none sets a console token, so FR-001's "no peer is not loopback" never meets them (run).
- **The binary's address line and the CLI tests' requests.**
  `crates/finder/tests/cli/session.rs:58` reads `finder console: http://`; it and the
  loopback hint stay first. Every CLI test that sends a request names `Host` with the
  console's IP address, so FR-004 admits it with the binary's token (run).
- **The support folder's files.** `crates/finder/tests/layout/main.rs:126` pins the six
  files of `crates/finder/tests/support/`; the rig is a module inside `mod.rs` (run).
- **The README's MCP count and sections.** `The imaging camera has sixteen tools of its
  own` (`crates/finder/README.md:88`) and `## MCP clients` / `## API` stay, so
  `crates/finder/tests/console/imaging_run_docs.rs:138` and
  `crates/finder/tests/mcp/finder_focus.rs` pass (run).
- **The browser specs on 8097, 8098 and 8099.** Every project opens the page at
  127.0.0.1, a loopback peer naming an IP literal, so from US3 on, with the binary's
  token, the gate asks them nothing (run); `pairing.spec.ts`'s wrong token counts under
  127.0.0.1 only at `/pair`.

## Decisions and the alternatives rejected

| Decision | Chosen | Rejected, and why |
|---|---|---|
| Where the gate sits | one middleware on `router`, after every route | per-route extractors (every route, and the next route, would need one) |
| How a browser presents it | a cookie set by the prompt | a header on every `fetch` (37 calls; `EventSource` and `<img>` cannot) |
| The prompt | the console's own HTML, always night | the React page (its files open to unpaired devices; `dist` changes) |
| MCP | the door asks after its stops; with MCP off, `/mcp` is gated as a page path | the gate before the door (would refuse a stop: 016's FR-012); `/mcp` passed by path alone (with `--no-mcp` it reached the page, 200, probe round 1) |
| The Host check | every request but Stop and, while MCP is on, `/mcp`, loopback's too | only another device's (a rebound name reads through the tunnel) |
| Stop | open to every origin | refusing a foreign `Origin` (changes 014's FR-020; a refused Stop) |
| The token's source | the state folder's file, generated once | a flag (process list), a PIN (guessable), per start (re-pair nightly) |
| Randomness | `/dev/urandom` with the standard library | `getrandom` (a `Cargo.lock` change for the same bytes) |
| The form | a hand-written `form_field` | `axum::Form` (a `Cargo.toml` feature for one field) |
| The limit | per address, sliding 60 s, 1,024 addresses | one count for all (a device on the Wi-Fi keeps the owner out) |
| A wrong cookie | cleared in the answer | kept (every request of an open page would count against the limit) |
| `--listen` | unchanged; the rig names the node's `.local` name | `0.0.0.0` by default |
| The test rig | a module in `support/mod.rs` | a file under `support/` (027's layout test); `super::pairing` (the constitution's `crate::support`) |
| The start line off loopback | a unit test of `listening_hint` | a CLI test on 0.0.0.0 (listens on every interface of the gate host) |

## User Story 1 - The gate

`pairing.rs` with the token, the count and the gate; `api.rs`'s field and `router`;
`lib.rs`'s module line and paragraph; the rig; the tests of spec.md US1.

## User Story 2 - The prompt and its cookie

`/pair`, only with a token, the prompt, the cookie set and cleared; `api.rs`'s
`routes(&state)`; `lib.rs`'s line; the tests of spec.md US2.

## User Story 3 - MCP, the binary's token and the browser test

`admits_mcp` and the door's three lines; `generate`, `grouped`, `load_or_create`,
`write_owner_only`, `listening_hint`; `main.rs`; the tests of spec.md US3, the browser
test among them.

## User Story 4 - The words

The crate README's rows, section and sentence; the main README, the safety page and the
console manual; the glossary; the tests of spec.md US4.

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
  `main`, and on 057 or 058 if either lands first: every `path:line` read again;
  `api.rs`'s field and `router` change placed against 055's field and `merge`; `lib.rs`'s
  module line after 055's; the README section, the glossary entries, the other pages'
  passages and the test binaries' `mod` lines placed after theirs, each in its
  alphabetical place (055 adds `mod nudge;`); every story's size measured again; the
  Refinement Record's last writers read again. Met at 936b140d: the 17 cites into changed files moved with
  their lines (none inside a changed hunk); `lib.rs`'s `pub mod pairing` and the test
  binaries' `mod` lines in their alphabetical places after 055's; the gate around the
  router 055 extended; the record's last writers name 054 US2 and 055 US1, US3 and US4;
  the stories measure 27,555, 22,041, 28,731 and 22,454 B on the replay, as before.
- The owner's ten decisions are answered. Met on 2026-11-28: each with its
  recommendation.
- A probe round from this text alone, folded. Met: probe round 1 built all four stories
  from the trio alone (`probe/056-r1`, tip 6fe4f173: 0 blocking, 16 clarity), and its
  fold is proven on that branch, every gate passing at e4335286 (§ What was run and what
  was read).
- The page half has run once on the re-anchored base (`CI=1 npm run e2e`). Met: on the
  probe's code replayed onto 936b140d, 39 of 39; it passed at e4335286, on f6d4f5fd.
- No operator-staged file is needed. Met: every story writes only code, tests and docs.

## Outside the factory

Written here and not run by any gate.

**The rig change.** The scope node's service for the console adds `--listen
0.0.0.0:8080` and `--allow-host <the node's .local name>` to its command line and is
restarted; its state folder then holds `console-token` after the first start. The other
ports stay off the network, since the console's token guards only 8080: the scope node's
config sets its `listen` to 127.0.0.1:50051, as the Pi 5 plan already has it
(`docs/safety-and-security.md:73` to `docs/safety-and-security.md:82`), and port 7624, which `indiserver` opens on
every interface with no authentication (`docs/safety-and-security.md:86` to `:88`), is
dropped by the firewall for every address but loopback. If the computer runs a firewall,
allow TCP 8080 from the local network only. Nothing else on the rig changes: the ssh
tunnel to 127.0.0.1:8080 and the Claude session work as before.

**At home, before the next field night**, with the phone on the house Wi-Fi: read the
token over ssh; open http://<the node's .local name>:8080 on the phone and note whether
it finds the name; type the token; turn Night on. **On the field night**, with the phone
on its own hotspot: open the same name, and note whether the phone finds it there and
whether anything asks for the token again; if not found, open the address the start line
printed and pair there. Check that the page opens, the live view runs and the night
switch holds; press one nudge arrow (055) and Stop. Then check the tunnel's page and an
MCP call from the laptop still work with no token. Notes go in the night's report;
nothing in the repository waits for them.

## Plan traps

They bind every story.

1. **The story sizes.** Probe round 1's commits from this text, each story's diff from
   the previous story's tip (`git diff <previous tip> -- .
   ':(exclude)crates/finder/web/dist'`), with the fold's changes in place at each tip:
   US1 27,558 B (26,485 B as built, 37,203 B at 1.35 times, 95 % of the line), US2
   22,035 B (21,834 B; 29,747 B), US3 28,739 B (28,481 B; 38,798 B, 98.7 % of the line,
   523 B under it) and US4 22,448 B (unchanged; 30,305 B). Every story is under 39,321 B
   at 1.35 times, so US3 stays whole. Review round 1's commits measured 26,644, 23,397,
   28,409 and 22,539 B. With the live stream's scenario in US1, US1 measured 28,636 B
   (38,659 B at 1.35 times, 98 % of the line), so it is US2's, where the cookie is set;
   the browser test is US3's, whose binary first has a token.
2. **An earlier story's test file is an existing test.** The rig is US1's, at the end of
   `crates/finder/tests/support/mod.rs`; later stories use it and never edit it, nor any
   other story's test, the unit tests in `pairing.rs` among them: US2 and US3 add their
   own `#[test]` functions there. A new file under `tests/support/` fails 027's layout
   test.
3. **`MockConnectInfo` stores itself.** Read the peer with the `ConnectInfo` extractor
   (`from_request_parts`), never with `extensions().get::<ConnectInfo<_>>()`.
4. **An unused item fails clippy.** `admits_mcp` and `listening_hint` arrive with their
   callers (US3), not before; `COOKIE_MAX_AGE_S` and `FORM_LIMIT` with `/pair` (US2).
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
12. **The gate hosts share one network.** No test binds anything but loopback: the CLI
    tests listen on 127.0.0.1:0, and `listening_hint` is a unit test.
13. **The Host check comes before the loopback pass.** A loopback request with a
    rebound name or no `Host` is refused (US1-S1); Stop, and `/mcp` while MCP is on, pass
    before it.
14. **The set cookie holds the compared form.** `test-console-token` is
    `TESTC0NS01ET0KEN` once O reads as 0 and L as 1; the scenarios' cookie
    `console_token=TESTCONSOLETOKEN` still matches.
15. **016's `session` helper names no host.** `crate::support::mcp::session` sends no
    `Host`, which FR-002 refuses from loopback once there is a token; US3's MCP tests read
    the session with `Host` localhost:8080 (run: 403 until they did).

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| A device on the Wi-Fi sniffs plain HTTP | it learns the token from any request | the README says private network only; TLS is a later decision (5 (b)) |
| Any web page on the network halts the mount | an imaging run or GoTo stops | the safe direction; 014's approval 2 keeps it (decision 1 (f)) |
| The phone cannot resolve `.local` on its hotspot | it pairs again by address, in the dark | pair at home; the start line prints the address; the field check notes it |
| INDI's 7624 or the link's 50051 answer the network | the token is not the weakest door | the rig keeps both to loopback (§ Outside the factory) |
| The owner loses the token | the phone cannot pair | the tunnel still works; read or replace the file |
| A browser keeps the cookie less than 400 days | the phone asks again | one prompt |
| WSL2 NAT peers are not loopback | a Windows browser needs the token | the README's WSL2 section; mirrored networking keeps loopback |
| 055, 057 or 058 change `api.rs`, `lib.rs` or the docs | the lines move | placed away from theirs; re-anchored before the flip |

## Refinement Record

Drafted on 2026-11-27 by the plate-solver operator seat's spec session against `main` at
f6d4f5fd, from a probe of that day on `probe/056`, and folded the same day after cynical
review round 1: every finding folded, each story rebuilt on the one before with both gates
passing at its own tip, every story's new tests failing on the story before it, and each
new or changed FR's check mutated once. Then probed from this text alone (round 1 on
`probe/056-r1`: 0 blocking, 16 clarity) and folded, each new step's test and the code
fixes proven on that branch (§ What was run and what was read). Read for it: the router,
MCP's door, 012's same-origin and host rules, 013's token comparison, 014's and 017's
Stop, the binary's start, the page's requests, DESIGN.md's fields, the test layout rules,
the constitution, the glossary, the public pages and the ADRs. Writes come from the task
text. Shared writes name the spec story that last wrote the same file on `main`;
`README.md`, `docs/safety-and-security.md` and `docs/guide/console.md` were last changed
by #371 and #360, not a spec's stories, and have no entry. Within the spec, each story's
files written by an earlier one are ordered by `depends_on_merged`.

The ten owner decisions of spec.md are answered, each with the recommendation this record
was written to, and the trio is re-anchored on `main` at 936b140d, so its `open_questions`
are empty and nothing is open. Nothing in this record is an owner
approval: those are in spec.md, and flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 936b140d3c0dda8879e28eb0efa3b5581f5a6bac
stories:
  US1:
    tests:
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "the gated console (support mcp state, a set clock, console token test-console-token) and the plain console: each read (GET /api/session, /api/health, /api/live, /api/live/preview.jpg?seq=1, /api/imaging/frames/1/preview.jpg, /) from 127.0.0.1, ::1 and ::ffff:127.0.0.1 with Host localhost:8080 and Host 127.0.0.1:8080 answers the status, content type and body the plain console gives 127.0.0.1; with Host rebound.example:8080 and with no Host 403 exactly {error: open the console at an IP address, localhost or an allowed host}; POST /api/stop from each with Host rebound.example:8080 200 exactly {stopped: true}; PUT /api/session {target M42} with Host localhost:8080 from each 200; GET /api/session from 127.0.0.2 with Host localhost:8080 200"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "the phone 192.168.1.30 with Host 192.168.1.20:8080 and no token: each read, GET /setup, GET /assets/index.js, GET /api/finder-camera/focus and PUT /api/session answer 401, those under /api/ (the focus route among them) exactly {error: this device is not paired: open the console's page and enter the console token}; GET /api/session with Host localhost:8080 through finder::router with no peer recorded the same 401; GET /mcp from the phone on the gated console with_mcp(McpConfig { enabled: false, ..Default::default() }) 401; the session read from 127.0.0.1 after equals the one before"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "from the phone with Cookie console_token=TESTCONSOLETOKEN, with Cookie theme=night; console_token=test-console-token, with Authorization Bearer test-console-token, with bearer test-c0ns0le-t0ken and with BEARER TESTCONSOLETOKEN: each read's answer equals the one 127.0.0.1 gets with Host 192.168.1.20:8080 and no token; PUT /api/session {target M42} with the cookie and Origin http://192.168.1.20:8080 answers 200 and target.messier is M42; GET /api/session from the phone with Authorization Bearer NOPE-NOPE and the cookie 401 exactly {error: that is not the console token}"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "GET /api/health from the phone at T + 0..9 s with Cookie console_token=NOPE-NOPE, Bearer NOPE-NOPE, Basic dGVzdA== and seven Bearer NOPE-NOPE: each 401 exactly {error: that is not the console token}; at T + 59.9 s the phone's GET /api/session with the cookie 429 exactly {error: too many wrong console tokens from this device: wait a minute} and its GET / with the bearer 429, the other phone 192.168.1.31's 200; at T + 60 s the phone's 200"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "with_allowed_hosts finder.example: POST /api/stop from the phone with Host rebound.example:8080 and no token answers 200 exactly {stopped: true}; GET /api/session with the cookie and Host rebound.example:8080, Host rebound.example and no Host answer 403 exactly {error: open the console at an IP address, localhost or an allowed host}; with Host finder.example:8080 200"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "GET /api/health with Bearer NOPE-NOPE at T once from each of fd00::1 to fd00::400 (1,024 addresses): each 401 exactly {error: that is not the console token}; fd00::401 with Bearer NOPE-NOPE 429 exactly {error: too many wrong console tokens from this device: wait a minute}, with the bearer 200; fd00::1 with the bearer 200; at T + 60 s fd00::401 with Bearer NOPE-NOPE 401 exactly {error: that is not the console token}"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/console/pairing.rs
        status: new
        disposition: add
        assertion: "GET /api/health with Bearer NOPE-NOPE from the phone at T + 0..9 s: each 401; GET /api/session with the cookie at T + 9 s 429; the clock set to T - 3,600 s, the same request 200"
        decision: "spec.md:US1-S7"
      - path: crates/finder/src/pairing.rs
        status: new
        disposition: add
        assertion: "unit: abcd-efgh-jkmn matches ABCDEFGHJKMN and ' abcd efgh jkmn ', not ABCDEFGHJKMP or ABCDEFGHJKM; 10ab-cdef-gh0j matches lOAB-CDEF-GHoJ and IOAB CDEF GHOJ; short, not/a/token and 65 letters are refused by ConsoleToken::new"
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
      - path: crates/finder/tests/motion/goto/sim.rs
        status: existing
        disposition: preserve
        assertion: "014's US6-S11: POST /api/stop with a foreign Origin and no body answers 200 and stops both axes"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "027's layout: tests/support holds goto.rs, live.rs, mcp.rs, mod.rs, stop.rs and tracker.rs and no tests"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a console without a console token answers every request as before this spec (FR-006)"
      - "POST /api/stop answers every peer as before this spec, from any origin and at any host (014 owner approval 2 and FR-020, 017 FR-004, FR-006)"
      - "012's check_origin and host_allowed are called, not changed; /mcp's door is not changed in US1"
    checks:
      development:
        - "cargo nextest run -p finder --locked --lib --test console -E 'test(/^pairing::/)'"
        - "cargo nextest run -p finder --locked --test mcp --test layout -E 'test(endpoint::) | binary(layout)'"
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
      - crates/finder/tests/support/mod.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 055-nudge-arrows-follow-the-frame
        other_story: US3
        disposition: merge_order
        reason: "056 depends_on_landed 055, whose US3 last changed api.rs (26a39791)"
      - path: crates/finder/src/lib.rs
        other_spec: 055-nudge-arrows-follow-the-frame
        other_story: US1
        disposition: merge_order
        reason: "056 depends_on_landed 055, whose US1 last changed lib.rs (0baeb8ad)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 054-focus-pause-gives-camera-back
        other_story: US2
        disposition: merge_order
        reason: "056 depends_on_landed 054, whose US2 last changed the console test main.rs (92e833da)"
      - path: crates/finder/tests/support/mod.rs
        other_spec: 028-finder-tests-do-less
        other_story: US5
        disposition: merge_order
        reason: "056 depends_on_landed 028, whose US5 last changed support/mod.rs (49bb210f)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/pairing_prompt.rs
        status: new
        disposition: add
        assertion: "the phone with no token: GET /, /setup and /assets/index.js answer 401, no Set-Cookie, text/html; charset=utf-8 holding <h1>Pair with the finder console</h1>, <p role=status>Pair this device: enter the console token.</p> and <form method=post action=/pair>; GET /pair the same with 200; GET /api/session 401 with the error this device is not paired: open the console's page and enter the console token; GET /pair with Host rebound.example:8080 403 exactly {error: open the console at an IP address, localhost or an allowed host}; on the plain console GET /pair answers what GET /pairx answers, 200, and a form with test-console-token to /pair what the same form to /pairx answers, 405"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/pairing_prompt.rs
        status: new
        disposition: add
        assertion: "POST /pair from the phone, form-urlencoded, Origin http://192.168.1.20:8080, token=test-CONSOLE+token: 303, Location /, Set-Cookie exactly console_token=TESTC0NS01ET0KEN; Path=/; Max-Age=34560000; HttpOnly; SameSite=Strict; each read with that cookie's name=value equals 127.0.0.1's answer with no token; token=TEST%2DCONSOLE%2DTOKEN 303; token= and 1,019 A (1,025 bytes) 413 with no Set-Cookie"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/pairing_prompt.rs
        status: new
        disposition: add
        assertion: "form NOPE-NOPE: the prompt That is not the console token. 401, no Set-Cookie; GET /api/session with Cookie console_token=NOPE 401 error that is not the console token, Set-Cookie exactly console_token=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict; GET / with it the prompt 401 and that Set-Cookie; GET / with Bearer NOPE the prompt 401, no Set-Cookie; six more forms 401; the right form and GET / with the cookie the prompt Too many wrong console tokens from this device: wait a minute and try again. 429, the form with no Set-Cookie; the other phone's form with Origin http://elsewhere.example 403 error writes require the console's own origin and an allowed host, no Set-Cookie"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/pairing_prompt.rs
        status: new
        disposition: add
        assertion: "GET /pair from the phone and 127.0.0.1: 200; at least 10 # in each page, each beginning #000000, #e00000, #d40000, #a80000 or #5a0000; none of rgb, hsl, white, <script, <img, <link, url(; holds <label for=token>Console token</label>, <input id=token name=token, min-height: 44px; border: 1px solid #e00000;, label { display: block; margin: 1rem 0 0.25rem; font-size: 0.875rem; font-weight: 600; color: #d40000; }, input { padding: 0 12px; background: transparent;, button { margin-top: 1rem; padding: 0 16px; background: #5a0000; font-weight: 560; } and html, body { margin: 0; background: #000000;"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/pairing_prompt.rs
        status: new
        disposition: add
        assertion: "the live console (support live state, a simulated node, console token test-console-token), its reports subscribed: a form with test-console-token from the phone, then GET /api/live with that answer's cookie 200 text/event-stream, its first chunk after the first report holding event: live and a data: line equal to that report serialized; GET /api/live/preview.jpg?seq=<its preview_seq> with the cookie equals 127.0.0.1's, 200 image/jpeg and exactly the loop's held JPEG; both GETs with no token 401 exactly {error: this device is not paired: open the console's page and enter the console token}"
        decision: "spec.md:US2-S5"
      - path: crates/finder/src/pairing.rs
        status: new
        disposition: add
        assertion: "unit: form_field decodes token=AB+CD%2DEF to AB CD-EF, finds token in x=1&token=Q, and gives none for x=1"
        decision: "spec.md:FR-008"
    preserved_behavior:
      - "US1's answers under /api/ and its limit are unchanged; only page paths gain the prompt"
      - "a console without a token answers GET /pair and POST /pair as before this spec (FR-006)"
      - "the React page and dist are not changed; playwright.config.ts is not edited"
    checks:
      development:
        - "cargo nextest run -p finder --locked --lib --test console -E 'test(/^pairing_prompt::/) | test(/^pairing::tests::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/pairing.rs
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/pairing_prompt.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
  US3:
    tests:
      - path: crates/finder/tests/mcp/pairing.rs
        status: new
        disposition: add
        assertion: "the MCP rig: get_session and set_target M42 from the phone with no token 401, www-authenticate exactly Bearer realm=finder console, body exactly {jsonrpc 2.0, error {code -32001, message MCP from another device needs the console token as a bearer token}}; with Bearer NOPE-NOPE 401 message that is not the console token; the session unchanged; with the bearer get_session 200 structuredContent equal to the session, with the cookie 200, and set_target 200, target.messier M42; 127.0.0.1 with no token 200; 203.0.113.7 with the bearer 403 and with no token 403; the phone on a console without a token 200"
        decision: "spec.md:US3-S1"
      - path: crates/finder/tests/mcp/pairing.rs
        status: new
        disposition: add
        assertion: "with the call log: stop from the phone with no token 401 with the no-token refusal and the log exactly mcp stop {} -> {\"stopped\":true} and a newline; ten get_session with Bearer NOPE-NOPE 401; then the bearer 429 message too many wrong console tokens from this device: wait a minute; GET /api/session from the phone with the cookie 429; the log in lower case holds no test-console-token"
        decision: "spec.md:US3-S2"
      - path: crates/finder/tests/cli/pairing.rs
        status: new
        disposition: add
        assertion: "the built finder on 127.0.0.1:0, a --state folder holding only console-token.tmp with junk: started and stopped, console-token is three groups of four of 0123456789ABCDEFGHJKMNPQRSTVWXYZ joined by - and one newline, mode 0600, console-token.tmp gone, stderr holds (to open it from a phone, run with --listen 0.0.0.0:8080); started again, GET /api/session with Host its address answers HTTP/1.1 200 OK, the file unchanged; neither stderr holds the token with or without dashes"
        decision: "spec.md:US3-S3"
      - path: crates/finder/tests/cli/pairing.rs
        status: new
        disposition: add
        assertion: "console-token my-own-console-token and a newline at mode 0644: kept as written at 0644, stderr holds <the file's path>: other users can read the console token; chmod 600 the file and, in lower case, not my-own-console-token; tiny7 and a newline: the start exits unsuccessfully within 60 s, stderr holds the line <the file's path>: a console token is 8 to 64 letters and digits; dashes and spaces are ignored, and neither tiny7 nor finder console: http://"
        decision: "spec.md:US3-S4"
      - path: crates/finder/src/pairing.rs
        status: new
        disposition: add
        assertion: "unit listening_hint with /state/console-token and a source giving 127.0.0.1 then 192.168.1.20, or none: 127.0.0.1:8080 and [::1]:8080 give (to open it from a phone, run with --listen 0.0.0.0:8080); 0.0.0.0:8080, [::]:8081 and 192.168.1.21:8080 (no source) give other devices open http://192.168.1.20:8080, http://192.168.1.20:8081 and http://192.168.1.21:8080 and pair with the console token in /state/console-token; 0.0.0.0:8080 with none gives other devices pair with the console token in /state/console-token"
        decision: "spec.md:US3-S5"
      - path: crates/finder/web/e2e/pairing.spec.ts
        status: new
        disposition: add
        assertion: "the default console at 412 x 915, scale 2: /pair 200 with the heading Pair with the finder console; with the field Console token focused, more than 10 computed colours of html, body and every element in body (color, background-color, border-top-color, border-bottom-color, outline-color, caret-color) other than rgba(0, 0, 0, 0), each rgb or rgba with green and blue 0; html background rgb(0, 0, 0); the field's outline rgb(224, 0, 0); field and button at least 44 px tall; after NOPE-NOPE-NOPE and Pair the status That is not the console token. under the same heading"
        decision: "spec.md:US3-S6"
      - path: crates/finder/src/pairing.rs
        status: new
        disposition: add
        assertion: "unit: grouped gives ABCD-EFGH-JKMN for abcdefghjkmn and MY0W-NC0N-S01E-T0KE-N for my-own-console-token; generate gives 12 symbols of the alphabet, two generations differ"
        decision: "spec.md:FR-012"
      - path: crates/finder/tests/cli/session.rs
        status: existing
        disposition: preserve
        assertion: "the binary's finder console: http:// line, session persistence and the clock, with a console-token now written in the state folder and every request naming the console's IP address"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the door carries out stops first and makes 016's Origin, Host and peer checks as before (016 FR-002, FR-003, FR-012; 017 FR-006)"
      - "--listen keeps its default and the loopback hint its words"
      - "every browser spec opens the page at 127.0.0.1 and passes with the binary's token"
    checks:
      development:
        - "cargo nextest run -p finder --locked --lib --test mcp --test cli -E 'test(/^pairing::/)'"
        - "npx playwright test --project=default e2e/pairing.spec.ts"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs (finder-page among them) and the field night of plan.md Outside the factory"
    writes:
      - crates/finder/src/main.rs
      - crates/finder/src/mcp/door.rs
      - crates/finder/src/pairing.rs
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/cli/pairing.rs
      - crates/finder/tests/mcp/main.rs
      - crates/finder/tests/mcp/pairing.rs
      - crates/finder/web/e2e/pairing.spec.ts
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
        other_spec: 055-nudge-arrows-follow-the-frame
        other_story: US4
        disposition: merge_order
        reason: "056 depends_on_landed 055, whose US4 last changed the MCP test main.rs (b1ccbddd)"
    open_questions: []
  US4:
    tests:
      - path: crates/finder/tests/console/pairing_docs.rs
        status: new
        disposition: add
        assertion: "crates/finder/README.md, every run of white space as one space: ### The console token after ### From a phone and before ## MCP clients, holding FR-013's fifteen sentences for it (Every device other than the console's own machine must present the console token before the console answers it. ... the token crosses the air unencrypted on every request.); FR-013's --listen and --allow-host rows; An MCP client on the console's own machine needs no token; another device's client sends the console token as `Authorization: Bearer <token>` (§ The console token). There is no console confirmation for MCP calls.; The imaging camera has sixteen tools of its own; no There is no token"
        decision: "spec.md:US4-S1"
      - path: crates/finder/tests/console/pairing_docs.rs
        status: new
        disposition: add
        assertion: "GLOSSARY.md holds MCP client with FR-014's words (a Claude session on the owner's laptop) and its _Avoid_ line, a blank line, then Console token with FR-014's words (Typing it into a browser pairs that device with the console, which is not the scope link's pairing of the console with a scope node.) and _Avoid_: password, PIN, API key, node token"
        decision: "spec.md:US4-S2"
      - path: crates/finder/tests/console/pairing_docs.rs
        status: new
        disposition: add
        assertion: "README.md, docs/safety-and-security.md and docs/guide/console.md, white space as one space: each holds FR-013's sentences for it (and MCP calls from its own machine carry no token.; **The console asks every other device for a token, over plain HTTP.**; **The phone asks for the console token once.** ...); none holds MCP calls carry no token., anyone on your network can move, The console has no login., `/mcp` has no token., anyone who can reach that port, Anyone on that network can then use, Otherwise writes are refused. or stateless and takes no token."
        decision: "spec.md:US4-S3"
      - path: crates/finder/tests/console/imaging_run_docs.rs
        status: existing
        disposition: preserve
        assertion: "the README's ## API rows and ## MCP clients' sixteen tools of its own, as before"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "every sentence of the five pages before this spec but the passages FR-013 and FR-014 replace stays; every other glossary entry stays"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console -E 'test(/^pairing_docs::/) | test(/^imaging_run_docs::/)'"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - GLOSSARY.md
      - README.md
      - crates/finder/README.md
      - crates/finder/tests/console/main.rs
      - crates/finder/tests/console/pairing_docs.rs
      - docs/guide/console.md
      - docs/safety-and-security.md
    no_write_reason: ""
    shared_writes:
      - path: GLOSSARY.md
        other_spec: 055-nudge-arrows-follow-the-frame
        other_story: US4
        disposition: merge_order
        reason: "056 depends_on_landed 055, whose US4 last changed GLOSSARY.md (b1ccbddd)"
      - path: crates/finder/README.md
        other_spec: 055-nudge-arrows-follow-the-frame
        other_story: US4
        disposition: merge_order
        reason: "056 depends_on_landed 055, whose US4 last changed the README (b1ccbddd)"
    open_questions: []
```
