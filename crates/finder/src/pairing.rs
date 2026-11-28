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

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use axum::extract::{ConnectInfo, FromRequestParts, Request, State};
use axum::http::{header, request::Parts, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::{api, AppState};

/// The browser cookie that presents the console token.
pub const COOKIE: &str = "console_token";
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

    fn matches(&self, supplied: &str) -> bool {
        scope_link::tokens_match(normalize(supplied).as_bytes(), self.normal.as_bytes())
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
    if peer.is_some_and(loopback) {
        return next.run(Request::from_parts(parts, body)).await;
    }
    let Some((supplied, _from_cookie)) = presented(&parts.headers) else {
        return api::ApiError::new(StatusCode::UNAUTHORIZED, UNPAIRED.1).into_response();
    };
    match pairing.check(
        peer.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        &supplied,
        state.clock.now(),
    ) {
        Check::Admitted => next.run(Request::from_parts(parts, body)).await,
        Check::Wrong => api::ApiError::new(StatusCode::UNAUTHORIZED, WRONG.1).into_response(),
        Check::Limited => {
            api::ApiError::new(StatusCode::TOO_MANY_REQUESTS, LIMITED.1).into_response()
        }
    }
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
}
