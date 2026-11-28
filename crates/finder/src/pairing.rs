//! The console token's gate for devices other than the console's own machine.
//!
//! With a token configured, requests must name an IP literal, localhost or an
//! allowed host before loopback peers pass. Other peers present the token as a
//! bearer header or the console cookie, compared through the scope link's token
//! comparison after Crockford's case and look-alike normalization. Missing peer
//! information is treated as another device. Stop always passes, and enabled MCP
//! keeps its own door. Without a console token every request passes unchanged.
//!
//! Wrong tokens are counted by peer address: ten in a sliding minute refuse any
//! presented token until the oldest expires. The count holds at most 1,024 peers,
//! drops times later than the console's clock, and never holds a lock over an await.
//!
//! The binary keeps its console token in `console-token` in its state folder,
//! creating an owner-only file at first start and keeping an owner's token as written.
//! It prints the file's path in the listening hint, never the token itself.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::body::to_bytes;
use axum::extract::{ConnectInfo, FromRequestParts, Request, State};
use axum::http::{header, request::Parts, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::{api, AppState};

/// The browser cookie that presents the console token.
pub const COOKIE: &str = "console_token";
/// The token file within the console's state folder.
pub const TOKEN_FILE: &str = "console-token";
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// The browser keeps the console token for 400 days.
pub const COOKIE_MAX_AGE_S: u64 = 400 * 24 * 60 * 60;
const FORM_LIMIT: usize = 1024;
/// Wrong tokens from one peer that fill the sliding window.
pub const WRONG_TOKENS: usize = 10;
/// The sliding window in seconds of the console's clock.
pub const WRONG_WINDOW_S: f64 = 60.0;
const MAX_PEERS: usize = 1024;

/// A console token, kept in its compared form and never formatted or serialized.
#[derive(Clone)]
pub struct ConsoleToken {
    normal: String,
}

impl ConsoleToken {
    /// Accept 8 to 64 ASCII letters and digits after normalization.
    pub fn new(text: &str) -> Result<Self, InvalidToken> {
        let normal = normalize(text);
        if !(8..=64).contains(&normal.len())
            || !normal.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err(InvalidToken);
        }
        Ok(Self { normal })
    }

    /// Generate twelve Crockford base32 symbols (60 bits) from `/dev/urandom`.
    pub fn generate() -> io::Result<Self> {
        let mut bytes = [0; 12];
        File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        // 256 is a multiple of 32, so masking keeps the distribution uniform.
        let normal = bytes
            .iter()
            .map(|byte| char::from(ALPHABET[(byte & 31) as usize]))
            .collect();
        Ok(Self { normal })
    }

    /// Format the compared token in groups of four symbols.
    pub fn grouped(&self) -> String {
        self.normal
            .as_bytes()
            .chunks(4)
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Read the owner's token unchanged, or atomically create an owner-only file.
    /// Errors and shared-file warnings name the path without disclosing its contents.
    pub fn load_or_create(path: &Path) -> Result<Self, String> {
        match fs::read_to_string(path) {
            Ok(text) => {
                let token =
                    Self::new(&text).map_err(|error| format!("{}: {error}", path.display()))?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let metadata = fs::metadata(path)
                        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
                    if metadata.permissions().mode() & 0o077 != 0 {
                        eprintln!(
                            "{}: other users can read the console token; chmod 600 the file",
                            path.display()
                        );
                    }
                }
                Ok(token)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let token = Self::generate()
                    .map_err(|error| format!("cannot generate console token: {error}"))?;
                write_owner_only(path, &format!("{}\n", token.grouped()))
                    .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
                Ok(token)
            }
            Err(error) => Err(format!("cannot read {}: {error}", path.display())),
        }
    }

    fn matches(&self, supplied: &str) -> bool {
        scope_link::tokens_match(normalize(supplied).as_bytes(), self.normal.as_bytes())
    }
}

fn write_owner_only(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_added_extension("tmp");
    match fs::remove_file(&temporary) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    fs::rename(temporary, path)
}

/// Tell the owner how another device reaches the console and finds its token file.
/// An unspecified listener uses Setup's outward-address source, which sends nothing.
pub fn listening_hint(
    bound: SocketAddr,
    addresses: &dyn crate::mcp::AddressSource,
    token_path: &Path,
) -> String {
    if loopback(bound.ip()) {
        return "(to open it from a phone, run with --listen 0.0.0.0:8080)".to_owned();
    }
    let address = if bound.ip().is_unspecified() {
        addresses.addresses().into_iter().find(|ip| !loopback(*ip))
    } else {
        Some(bound.ip())
    };
    match address {
        Some(ip) => format!(
            "other devices open http://{} and pair with the console token in {}",
            SocketAddr::new(ip, bound.port()),
            token_path.display()
        ),
        None => format!(
            "other devices pair with the console token in {}",
            token_path.display()
        ),
    }
}

fn normalize(text: &str) -> String {
    text.chars()
        .filter(|ch| *ch != '-' && !ch.is_whitespace())
        .map(|ch| match ch.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect()
}

/// A console token outside the accepted length or alphabet.
#[derive(Debug)]
pub struct InvalidToken;

impl fmt::Display for InvalidToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "a console token is 8 to 64 letters and digits; dashes and spaces are ignored",
        )
    }
}
impl std::error::Error for InvalidToken {}

pub(crate) struct Pairing {
    token: ConsoleToken,
    wrong: Mutex<HashMap<IpAddr, VecDeque<f64>>>,
}

enum Check {
    Admitted,
    Wrong,
    Limited,
}

impl Pairing {
    fn check(&self, peer: IpAddr, supplied: &str, now: f64) -> Check {
        let mut wrong = self.wrong.lock().unwrap();
        wrong.retain(|_, times| {
            times.retain(|at| *at <= now && now - at < WRONG_WINDOW_S);
            !times.is_empty()
        });
        if wrong
            .get(&peer)
            .is_some_and(|times| times.len() >= WRONG_TOKENS)
        {
            return Check::Limited;
        }
        if self.token.matches(supplied) {
            return Check::Admitted;
        }
        if !wrong.contains_key(&peer) && wrong.len() >= MAX_PEERS {
            return Check::Limited;
        }
        wrong.entry(peer).or_default().push_back(now);
        Check::Wrong
    }
}

impl AppState {
    /// Require this token from devices other than loopback peers at allowed hosts.
    /// Cloned state shares the wrong-token count and the console's clock.
    pub fn with_console_token(mut self, token: ConsoleToken) -> Self {
        self.pairing = Some(Arc::new(Pairing {
            token,
            wrong: Mutex::new(HashMap::new()),
        }));
        self
    }
}

fn loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_loopback(),
        IpAddr::V6(ip) => {
            ip.is_loopback() || ip.to_ipv4_mapped().is_some_and(|ip| ip.is_loopback())
        }
    }
}

async fn peer(parts: &mut Parts) -> Option<IpAddr> {
    ConnectInfo::<SocketAddr>::from_request_parts(parts, &())
        .await
        .ok()
        .map(|peer| peer.0.ip())
}

fn presented(headers: &HeaderMap) -> Option<(String, bool)> {
    if let Some(authorization) = headers.get(header::AUTHORIZATION) {
        let token = authorization
            .to_str()
            .ok()
            .and_then(|text| text.split_once(' '))
            .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
            .map_or("", |(_, token)| token);
        return Some((token.to_owned(), false));
    }
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|pair| {
            let (name, value) = pair.trim().split_once('=')?;
            (name == COOKIE).then(|| (value.to_owned(), true))
        })
}

const UNPAIRED: (&str, &str) = (
    "Pair this device: enter the console token.",
    "this device is not paired: open the console's page and enter the console token",
);
const WRONG: (&str, &str) = (
    "That is not the console token.",
    "that is not the console token",
);
const LIMITED: (&str, &str) = (
    "Too many wrong console tokens from this device: wait a minute and try again.",
    "too many wrong console tokens from this device: wait a minute",
);

/// MCP calls this only after carrying out Stop and checking Origin, Host and peer.
pub(crate) async fn admits_mcp(parts: &mut Parts, state: &AppState) -> Option<Response> {
    let pairing = state.pairing.as_ref()?;
    let peer = peer(parts).await;
    if peer.is_some_and(loopback) {
        return None;
    }
    let (status, message) = match presented(&parts.headers) {
        None => (
            StatusCode::UNAUTHORIZED,
            "MCP from another device needs the console token as a bearer token",
        ),
        Some((supplied, _)) => match pairing.check(
            peer.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            &supplied,
            state.clock.now(),
        ) {
            Check::Admitted => return None,
            Check::Wrong => (StatusCode::UNAUTHORIZED, WRONG.1),
            Check::Limited => (StatusCode::TOO_MANY_REQUESTS, LIMITED.1),
        },
    };
    Some(
        (
            status,
            [(header::WWW_AUTHENTICATE, "Bearer realm=\"finder console\"")],
            Json(json!({"jsonrpc":"2.0", "error":{"code":-32001, "message":message}})),
        )
            .into_response(),
    )
}

pub(crate) async fn gate(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let Some(pairing) = &state.pairing else {
        return next.run(request).await;
    };
    let (mut parts, body) = request.into_parts();
    let path = parts.uri.path();
    if path == "/api/stop" || (path == "/mcp" && state.mcp.enabled) {
        return next.run(Request::from_parts(parts, body)).await;
    }
    if !api::host_allowed(&parts, &state.allowed_hosts) {
        return api::ApiError::new(
            StatusCode::FORBIDDEN,
            "open the console at an IP address, localhost or an allowed host",
        )
        .into_response();
    }
    let peer = peer(&mut parts).await;
    if peer.is_some_and(loopback) || parts.uri.path() == "/pair" {
        return next.run(Request::from_parts(parts, body)).await;
    }
    let Some((supplied, from_cookie)) = presented(&parts.headers) else {
        return refusal(parts.uri.path(), StatusCode::UNAUTHORIZED, UNPAIRED);
    };
    match pairing.check(
        peer.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        &supplied,
        state.clock.now(),
    ) {
        Check::Admitted => next.run(Request::from_parts(parts, body)).await,
        Check::Wrong => {
            let mut response = refusal(parts.uri.path(), StatusCode::UNAUTHORIZED, WRONG);
            if from_cookie {
                response.headers_mut().insert(
                    header::SET_COOKIE,
                    HeaderValue::from_static(
                        "console_token=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict",
                    ),
                );
            }
            response
        }
        Check::Limited => refusal(parts.uri.path(), StatusCode::TOO_MANY_REQUESTS, LIMITED),
    }
}

fn refusal(path: &str, status: StatusCode, words: (&str, &str)) -> Response {
    if path.starts_with("/api/") {
        api::ApiError::new(status, words.1).into_response()
    } else {
        (status, prompt_html(words.0)).into_response()
    }
}

pub(crate) fn routes(state: &AppState) -> Router<AppState> {
    if state.pairing.is_some() {
        Router::new().route("/pair", get(prompt).post(pair))
    } else {
        Router::new()
    }
}

async fn prompt() -> Html<String> {
    prompt_html(UNPAIRED.0)
}

async fn pair(State(state): State<AppState>, request: Request) -> Response {
    let (mut parts, body) = request.into_parts();
    if let Err(error) = api::check_origin(&parts, &state.allowed_hosts) {
        return error.into_response();
    }
    let Ok(body) = to_bytes(body, FORM_LIMIT).await else {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let supplied = form_field(&body, "token").unwrap_or_default();
    let peer = peer(&mut parts)
        .await
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let pairing = state.pairing.as_ref().expect("pair route requires a token");
    match pairing.check(peer, &supplied, state.clock.now()) {
        Check::Admitted => {
            let cookie = format!(
                "{COOKIE}={}; Path=/; Max-Age={COOKIE_MAX_AGE_S}; HttpOnly; SameSite=Strict",
                pairing.token.normal
            );
            (
                StatusCode::SEE_OTHER,
                [(header::LOCATION, "/"), (header::SET_COOKIE, &cookie)],
            )
                .into_response()
        }
        Check::Wrong => (StatusCode::UNAUTHORIZED, prompt_html(WRONG.0)).into_response(),
        Check::Limited => (StatusCode::TOO_MANY_REQUESTS, prompt_html(LIMITED.0)).into_response(),
    }
}

fn form_field(body: &[u8], name: &str) -> Option<String> {
    fn decode(text: &str) -> Option<String> {
        let mut decoded = Vec::with_capacity(text.len());
        let mut bytes = text.bytes();
        while let Some(byte) = bytes.next() {
            decoded.push(match byte {
                b'+' => b' ',
                b'%' => {
                    let high = (bytes.next()? as char).to_digit(16)?;
                    let low = (bytes.next()? as char).to_digit(16)?;
                    ((high << 4) | low) as u8
                }
                other => other,
            });
        }
        String::from_utf8(decoded).ok()
    }

    std::str::from_utf8(body)
        .ok()?
        .split('&')
        .find_map(|field| {
            let (key, value) = field.split_once('=')?;
            (decode(key)? == name).then(|| decode(value)).flatten()
        })
}

fn prompt_html(message: &str) -> Html<String> {
    Html(format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<title>Pair with the finder console</title>
<style>
html, body {{ margin: 0; background: #000000; color: #e00000; font: 540 16px/1.5 system-ui, sans-serif; }}
main {{ max-width: 24rem; margin: 0 auto; padding: 2rem 1rem; }}
h1 {{ font-size: 1.25rem; font-weight: 600; margin: 0 0 1rem; }}
p {{ color: #d40000; }}
label {{ display: block; margin: 1rem 0 0.25rem; font-size: 0.875rem; font-weight: 600; color: #d40000; }}
input, button {{ box-sizing: border-box; width: 100%; min-height: 44px; border: 1px solid #e00000; border-radius: 0; font: inherit; color: #e00000; caret-color: #e00000; }}
input {{ padding: 0 12px; background: transparent; letter-spacing: 0.1em; text-transform: uppercase; }}
button {{ margin-top: 1rem; padding: 0 16px; background: #5a0000; font-weight: 560; }}
input:focus, button:focus {{ outline: 2px solid #e00000; outline-offset: 2px; }}
::selection {{ background: #5a0000; color: #e00000; }}
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
"##
    ))
}

#[cfg(test)]
mod tests {
    use super::ConsoleToken;

    /// 056 FR-001's worked spellings, exact comparison and validation outcomes.
    #[test]
    fn token_normalization_and_validation() {
        let token = ConsoleToken::new("abcd-efgh-jkmn").unwrap();
        assert!(token.matches("ABCDEFGHJKMN"));
        assert!(token.matches(" abcd efgh jkmn "));
        assert!(!token.matches("ABCDEFGHJKMP"));
        assert!(!token.matches("ABCDEFGHJKM"));
        let token = ConsoleToken::new("10ab-cdef-gh0j").unwrap();
        assert!(token.matches("lOAB-CDEF-GHoJ"));
        assert!(token.matches("IOAB CDEF GHOJ"));
        for invalid in ["short", "not/a/token", &"A".repeat(65)] {
            assert!(ConsoleToken::new(invalid).is_err());
        }
    }

    /// FR-008's worked form decoding and missing-field outcomes, exactly.
    #[test]
    fn form_field_decodes_and_finds_the_token() {
        assert_eq!(
            super::form_field(b"token=AB+CD%2DEF", "token").as_deref(),
            Some("AB CD-EF")
        );
        assert_eq!(
            super::form_field(b"x=1&token=Q", "token").as_deref(),
            Some("Q")
        );
        assert_eq!(super::form_field(b"x=1", "token"), None);
        for invalid in [b"token=%zz".as_slice(), b"token=%", b"token=%ff"] {
            assert_eq!(super::form_field(invalid, "token"), None);
        }
    }

    /// FR-012's worked spellings, grouped exactly without changing comparison.
    #[test]
    fn grouped_uses_four_symbols_per_group() {
        assert_eq!(
            ConsoleToken::new("abcdefghjkmn").unwrap().grouped(),
            "ABCD-EFGH-JKMN"
        );
        assert_eq!(
            ConsoleToken::new("my-own-console-token").unwrap().grouped(),
            "MY0W-NC0N-S01E-T0KE-N"
        );
    }

    /// FR-012's 12-symbol Crockford alphabet; generated values are never printed.
    #[test]
    fn generated_tokens_have_twelve_random_crockford_symbols() {
        let first = ConsoleToken::generate().unwrap();
        let second = ConsoleToken::generate().unwrap();
        for token in [&first, &second] {
            assert_eq!(token.normal.len(), 12);
            assert!(token
                .normal
                .bytes()
                .all(|byte| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&byte)));
        }
        assert!(
            first.normal != second.normal,
            "independent generations differ"
        );
    }

    /// US3-S5's fixed address lists and exact listening lines, no tolerance.
    #[test]
    fn listening_hints_use_the_bound_address_or_first_outward_address() {
        use std::net::IpAddr;
        use std::path::Path;

        struct Addresses(Vec<IpAddr>);
        impl crate::mcp::AddressSource for Addresses {
            fn addresses(&self) -> Vec<IpAddr> {
                self.0.clone()
            }
        }
        struct Unused;
        impl crate::mcp::AddressSource for Unused {
            fn addresses(&self) -> Vec<IpAddr> {
                panic!("loopback needs no outward address")
            }
        }
        let source = Addresses(vec![
            "127.0.0.1".parse().unwrap(),
            "192.168.1.20".parse().unwrap(),
        ]);
        let none = Addresses(vec![]);
        let path = Path::new("/state/console-token");
        for bound in ["127.0.0.1:8080", "[::1]:8080"] {
            for addresses in [&source as &dyn crate::mcp::AddressSource, &Unused] {
                assert_eq!(
                    super::listening_hint(bound.parse().unwrap(), addresses, path),
                    "(to open it from a phone, run with --listen 0.0.0.0:8080)"
                );
            }
        }
        for (bound, addresses, advertised) in [
            ("0.0.0.0:8080", &source, "192.168.1.20:8080"),
            ("[::]:8081", &source, "192.168.1.20:8081"),
            ("192.168.1.21:8080", &none, "192.168.1.21:8080"),
        ] {
            assert_eq!(super::listening_hint(bound.parse().unwrap(), addresses, path),
                format!("other devices open http://{advertised} and pair with the console token in /state/console-token"));
        }
        assert_eq!(
            super::listening_hint("0.0.0.0:8080".parse().unwrap(), &none, path),
            "other devices pair with the console token in /state/console-token"
        );
    }
}
