//! US1's seven scenarios. Oracles: the plain console and loopback's answers,
//! and the specification's exact statuses, JSON and clock boundaries (zero tolerance).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use serde_json::json;

use crate::support::pairing::*;

const BEARER: (&str, &str) = ("authorization", "Bearer test-console-token");
const WRONG_BEARER: (&str, &str) = ("authorization", "Bearer NOPE-NOPE");

fn edit(host: &str) -> Request<Body> {
    Request::put("/api/session")
        .header("host", host)
        .header("content-type", "application/json")
        .body(Body::from(r#"{"target":"M42"}"#))
        .unwrap()
}

async fn error(response: Response, status: u16, words: &str) {
    assert_eq!(response.status(), status);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_eq!(json_of(response).await, json!({"error": words}));
}

#[tokio::test]
async fn us1_s1_loopback_keeps_plain_answers_at_the_consoles_own_host() {
    for peer in ["127.0.0.1", "::1", "::ffff:127.0.0.1"] {
        let plain = from(crate::support::mcp::state(), "127.0.0.1");
        let app = from(gated(clock()), peer);
        for path in READS {
            for host in ["localhost:8080", "127.0.0.1:8080"] {
                assert_eq!(
                    answer(send(&app, get(path, Some(host), None)).await).await,
                    answer(send(&plain, get(path, Some(host), None)).await).await,
                    "{peer} {path} {host}"
                );
            }
            for host in [Some("rebound.example:8080"), None] {
                error(send(&app, get(path, host, None)).await, 403, BAD_HOST).await;
            }
        }
        let stop = Request::post("/api/stop")
            .header("host", "rebound.example:8080")
            .body(Body::empty())
            .unwrap();
        let response = send(&app, stop).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_of(response).await, json!({"stopped": true}));
        assert_eq!(
            send(&app, edit("localhost:8080")).await.status(),
            StatusCode::OK
        );
    }
    let app = from(gated(clock()), "127.0.0.2");
    assert_eq!(
        send(&app, get("/api/session", Some("localhost:8080"), None))
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn us1_s2_unpaired_and_unrecorded_peers_never_reach_a_route() {
    let state = gated(clock());
    let local = from(state.clone(), "127.0.0.1");
    let phone = from(state.clone(), PHONE);
    let before = answer(send(&local, get("/api/session", Some(HOST), None)).await).await;
    for path in
        READS
            .iter()
            .copied()
            .chain(["/setup", "/assets/index.js", "/api/finder-camera/focus"])
    {
        let response = send(&phone, get(path, Some(HOST), None)).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
        if path.starts_with("/api/") {
            error(response, 401, UNPAIRED).await;
        }
    }
    error(send(&phone, edit(HOST)).await, 401, UNPAIRED).await;
    error(
        send(
            &finder::router(state.clone()),
            get("/api/session", Some("localhost:8080"), None),
        )
        .await,
        401,
        UNPAIRED,
    )
    .await;
    let off = from(
        state.with_mcp(finder::mcp::McpConfig {
            enabled: false,
            ..Default::default()
        }),
        PHONE,
    );
    assert_eq!(
        send(&off, get("/mcp", Some(HOST), None)).await.status(),
        401
    );
    assert_eq!(
        answer(send(&local, get("/api/session", Some(HOST), None)).await).await,
        before
    );
}

#[tokio::test]
async fn us1_s3_cookie_and_bearer_admit_the_phone_with_header_precedence() {
    let state = gated(clock());
    let local = from(state.clone(), "127.0.0.1");
    let phone = from(state, PHONE);
    for path in READS {
        let expected = answer(send(&local, get(path, Some(HOST), None)).await).await;
        for header in [
            ("cookie", COOKIE),
            ("cookie", "theme=night; console_token=test-console-token"),
            BEARER,
            ("authorization", "bearer test-c0ns0le-t0ken"),
            ("authorization", "BEARER TESTCONSOLETOKEN"),
        ] {
            assert_eq!(
                answer(send(&phone, get(path, Some(HOST), Some(header))).await).await,
                expected,
                "{path}"
            );
        }
    }
    let mut request = edit(HOST);
    request
        .headers_mut()
        .insert("cookie", COOKIE.parse().unwrap());
    request
        .headers_mut()
        .insert("origin", ORIGIN.parse().unwrap());
    assert_eq!(send(&phone, request).await.status(), StatusCode::OK);
    let session = json_of(send(&local, get("/api/session", Some(HOST), None)).await).await;
    assert_eq!(session["target"]["messier"], "M42");
    let mut request = get("/api/session", Some(HOST), Some(WRONG_BEARER));
    request
        .headers_mut()
        .insert("cookie", COOKIE.parse().unwrap());
    error(send(&phone, request).await, 401, WRONG).await;
}

#[tokio::test]
async fn us1_s4_ten_wrong_tokens_limit_only_their_peer_for_sixty_seconds() {
    let time = clock();
    let state = gated(time.clone());
    let phone = from(state.clone(), PHONE);
    let other = from(state, OTHER_PHONE);
    for (n, header) in [
        ("cookie", "console_token=NOPE-NOPE"),
        WRONG_BEARER,
        ("authorization", "Basic dGVzdA=="),
    ]
    .into_iter()
    .chain(std::iter::repeat_n(WRONG_BEARER, 7))
    .enumerate()
    {
        time.set(T + n as f64);
        error(
            send(&phone, get("/api/health", Some(HOST), Some(header))).await,
            401,
            WRONG,
        )
        .await;
    }
    time.set(T + 59.9);
    error(
        send(
            &phone,
            get("/api/session", Some(HOST), Some(("cookie", COOKIE))),
        )
        .await,
        429,
        LIMITED,
    )
    .await;
    assert_eq!(
        send(&phone, get("/", Some(HOST), Some(BEARER)))
            .await
            .status(),
        429
    );
    assert_eq!(
        send(
            &other,
            get("/api/session", Some(HOST), Some(("cookie", COOKIE)))
        )
        .await
        .status(),
        200
    );
    // The limit applies to presented tokens; a missing token is still unpaired.
    error(
        send(&phone, get("/api/session", Some(HOST), None)).await,
        401,
        UNPAIRED,
    )
    .await;
    time.set(T + 60.0);
    assert_eq!(
        send(
            &phone,
            get("/api/session", Some(HOST), Some(("cookie", COOKIE)))
        )
        .await
        .status(),
        200
    );
}

#[tokio::test]
async fn us1_s5_host_check_precedes_tokens_but_stop_always_passes() {
    let phone = from(
        gated(clock()).with_allowed_hosts(["finder.example".to_owned()]),
        PHONE,
    );
    let stop = Request::post("/api/stop")
        .header("host", "rebound.example:8080")
        .body(Body::empty())
        .unwrap();
    let response = send(&phone, stop).await;
    assert_eq!(response.status(), 200);
    assert_eq!(json_of(response).await, json!({"stopped": true}));
    for host in [Some("rebound.example:8080"), Some("rebound.example"), None] {
        error(
            send(&phone, get("/api/session", host, Some(("cookie", COOKIE)))).await,
            403,
            BAD_HOST,
        )
        .await;
        error(
            send(&phone, get("/api/session", host, Some(WRONG_BEARER))).await,
            403,
            BAD_HOST,
        )
        .await;
    }
    assert_eq!(
        send(
            &phone,
            get(
                "/api/session",
                Some("finder.example:8080"),
                Some(("cookie", COOKIE))
            )
        )
        .await
        .status(),
        200
    );
    let stop = Request::post("/api/stop")
        .header("host", "rebound.example:8080")
        .header("origin", "http://evil.example")
        .header("authorization", "Bearer NOPE-NOPE")
        .body(Body::empty())
        .unwrap();
    let response = send(&phone, stop).await;
    assert_eq!(response.status(), 200);
    assert_eq!(json_of(response).await, json!({"stopped": true}));
    assert_eq!(
        send(&phone, get("/api/stop", None, None)).await.status(),
        405
    );
}

#[tokio::test]
async fn us1_s6_at_most_1024_peers_are_counted_and_expired_peers_are_removed() {
    let time = clock();
    let state = gated(time.clone());
    for n in 1..=1024 {
        let app = from(state.clone(), &format!("fd00::{n:x}"));
        error(
            send(&app, get("/api/health", Some(HOST), Some(WRONG_BEARER))).await,
            401,
            WRONG,
        )
        .await;
    }
    let new_peer = from(state.clone(), "fd00::401");
    error(
        send(
            &new_peer,
            get("/api/health", Some(HOST), Some(WRONG_BEARER)),
        )
        .await,
        429,
        LIMITED,
    )
    .await;
    assert_eq!(
        send(&new_peer, get("/api/health", Some(HOST), Some(BEARER)))
            .await
            .status(),
        200
    );
    let first_peer = from(state, "fd00::1");
    assert_eq!(
        send(&first_peer, get("/api/health", Some(HOST), Some(BEARER)))
            .await
            .status(),
        200
    );
    time.set(T + 60.0);
    error(
        send(
            &new_peer,
            get("/api/health", Some(HOST), Some(WRONG_BEARER)),
        )
        .await,
        401,
        WRONG,
    )
    .await;
}

#[tokio::test]
async fn us1_s7_a_clock_step_back_drops_wrong_tokens_from_the_future() {
    let time = clock();
    let phone = from(gated(time.clone()), PHONE);
    for n in 0..10 {
        time.set(T + f64::from(n));
        error(
            send(&phone, get("/api/health", Some(HOST), Some(WRONG_BEARER))).await,
            401,
            WRONG,
        )
        .await;
    }
    error(
        send(
            &phone,
            get("/api/session", Some(HOST), Some(("cookie", COOKIE))),
        )
        .await,
        429,
        LIMITED,
    )
    .await;
    time.set(T - 3600.0);
    assert_eq!(
        send(
            &phone,
            get("/api/session", Some(HOST), Some(("cookie", COOKIE)))
        )
        .await
        .status(),
        200
    );
}
