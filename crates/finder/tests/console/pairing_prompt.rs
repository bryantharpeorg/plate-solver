//! US2's five scenarios. Oracles: the specification's words, cookies and night
//! colours, plain/loopback answers, and the live loop's report and held JPEG.
//! All comparisons are exact; time comes from the rig or the paused simulator.

use axum::body::Body;
use axum::http::Request;
use axum::response::Response;
use futures_util::StreamExt;
use serde_json::json;
use std::time::Duration;

use crate::support::live;
use crate::support::pairing::*;

const SET_COOKIE: &str =
    "console_token=TESTC0NS01ET0KEN; Path=/; Max-Age=34560000; HttpOnly; SameSite=Strict";
const CLEARED_COOKIE: &str = "console_token=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict";
const ASK: &str = "Pair this device: enter the console token.";
const WRONG_PROMPT: &str = "That is not the console token.";
const LIMITED_PROMPT: &str =
    "Too many wrong console tokens from this device: wait a minute and try again.";

fn form(path: &str, body: impl Into<Body>, origin: &str) -> Request<Body> {
    Request::post(path)
        .header("host", HOST)
        .header("origin", origin)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body.into())
        .unwrap()
}

async fn prompt(response: Response, status: u16, words: &str) -> String {
    assert_eq!(response.status(), status);
    assert_eq!(
        response.headers()["content-type"],
        "text/html; charset=utf-8"
    );
    let (_, _, bytes) = answer(response).await;
    let text = String::from_utf8(bytes).unwrap();
    for expected in [
        "<h1>Pair with the finder console</h1>".to_owned(),
        format!("<p role=\"status\">{words}</p>"),
        "<form method=\"post\" action=\"/pair\">".to_owned(),
    ] {
        assert!(text.contains(&expected), "missing {expected}");
    }
    assert!(!text.contains(TOKEN));
    assert!(!text.contains("TESTC0NS01ET0KEN"));
    text
}

async fn error(response: Response, status: u16, words: &str) {
    assert_eq!(response.status(), status);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_eq!(json_of(response).await, json!({"error": words}));
}

#[tokio::test]
async fn us2_s1_pages_ask_api_refuses_and_plain_console_has_no_pair_route() {
    let phone = from(gated(clock()), PHONE);
    for (path, status) in [
        ("/", 401),
        ("/setup", 401),
        ("/assets/index.js", 401),
        ("/pair", 200),
    ] {
        let response = send(&phone, get(path, Some(HOST), None)).await;
        assert!(!response.headers().contains_key("set-cookie"));
        prompt(response, status, ASK).await;
    }
    error(
        send(&phone, get("/api/session", Some(HOST), None)).await,
        401,
        UNPAIRED,
    )
    .await;
    error(
        send(&phone, get("/pair", Some("rebound.example:8080"), None)).await,
        403,
        BAD_HOST,
    )
    .await;

    let plain = from(crate::support::mcp::state(), PHONE);
    let pair = send(&plain, get("/pair", Some(HOST), None)).await;
    assert_eq!(pair.status(), 200);
    assert_eq!(
        answer(pair).await,
        answer(send(&plain, get("/pairx", Some(HOST), None)).await).await
    );
    let pair = send(&plain, form("/pair", "token=test-console-token", ORIGIN)).await;
    assert_eq!(pair.status(), 405);
    assert_eq!(
        answer(pair).await,
        answer(send(&plain, form("/pairx", "token=test-console-token", ORIGIN)).await).await
    );
}

#[tokio::test]
async fn us2_s2_decoded_form_sets_compared_cookie_and_opens_every_read() {
    let state = gated(clock());
    let phone = from(state.clone(), PHONE);
    let local = from(state, "127.0.0.1");
    let response = send(&phone, form("/pair", "token=test-CONSOLE+token", ORIGIN)).await;
    assert_eq!(response.status(), 303);
    assert_eq!(response.headers()["location"], "/");
    assert_eq!(response.headers()["set-cookie"], SET_COOKIE);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    assert!(answer(response).await.2.is_empty());
    for path in READS {
        assert_eq!(
            answer(send(&phone, get(path, Some(HOST), Some(("cookie", &cookie)))).await).await,
            answer(send(&local, get(path, Some(HOST), None)).await).await,
            "{path}"
        );
    }
    let response = send(
        &phone,
        form("/pair", "token=TEST%2DCONSOLE%2DTOKEN", ORIGIN),
    )
    .await;
    assert_eq!(response.status(), 303);
    assert_eq!(response.headers()["set-cookie"], SET_COOKIE);

    // FR-008 reads form data whatever Content-Type says (owner decision 10).
    let mut request = form("/pair", "token=test-console-token", ORIGIN);
    request
        .headers_mut()
        .insert("content-type", "text/plain".parse().unwrap());
    assert_eq!(send(&phone, request).await.status(), 303);

    let body = format!("token={}", "A".repeat(1_019));
    assert_eq!(body.len(), 1_025);
    let response = send(&phone, form("/pair", body, ORIGIN)).await;
    assert_eq!(response.status(), 413);
    assert!(!response.headers().contains_key("set-cookie"));
}

#[tokio::test]
async fn us2_s3_wrong_cookie_is_cleared_and_forms_share_the_peer_limit() {
    let state = gated(clock());
    let phone = from(state.clone(), PHONE);
    let response = send(&phone, form("/pair", "token=NOPE-NOPE", ORIGIN)).await;
    assert!(!response.headers().contains_key("set-cookie"));
    prompt(response, 401, WRONG_PROMPT).await;

    let wrong_cookie = Some(("cookie", "console_token=NOPE"));
    let response = send(&phone, get("/api/session", Some(HOST), wrong_cookie)).await;
    assert_eq!(response.headers()["set-cookie"], CLEARED_COOKIE);
    error(response, 401, WRONG).await;
    let response = send(&phone, get("/", Some(HOST), wrong_cookie)).await;
    assert_eq!(response.headers()["set-cookie"], CLEARED_COOKIE);
    prompt(response, 401, WRONG_PROMPT).await;
    let response = send(
        &phone,
        get("/", Some(HOST), Some(("authorization", "Bearer NOPE"))),
    )
    .await;
    assert!(!response.headers().contains_key("set-cookie"));
    prompt(response, 401, WRONG_PROMPT).await;
    for _ in 0..6 {
        let response = send(&phone, form("/pair", "token=NOPE-NOPE", ORIGIN)).await;
        assert!(!response.headers().contains_key("set-cookie"));
        prompt(response, 401, WRONG_PROMPT).await;
    }
    let response = send(&phone, form("/pair", "token=test-console-token", ORIGIN)).await;
    assert!(!response.headers().contains_key("set-cookie"));
    prompt(response, 429, LIMITED_PROMPT).await;
    prompt(
        send(&phone, get("/", Some(HOST), Some(("cookie", COOKIE)))).await,
        429,
        LIMITED_PROMPT,
    )
    .await;

    let other = from(state, OTHER_PHONE);
    let response = send(
        &other,
        form(
            "/pair",
            "token=test-console-token",
            "http://elsewhere.example",
        ),
    )
    .await;
    assert!(!response.headers().contains_key("set-cookie"));
    error(
        response,
        403,
        "writes require the console's own origin and an allowed host",
    )
    .await;
}

#[tokio::test]
async fn us2_s4_prompt_uses_only_designs_night_colours_and_controls() {
    let state = gated(clock());
    for peer in [PHONE, "127.0.0.1"] {
        let app = from(state.clone(), peer);
        let text = prompt(send(&app, get("/pair", Some(HOST), None)).await, 200, ASK).await;
        let colours: Vec<_> = text.match_indices('#').map(|(at, _)| &text[at..]).collect();
        assert!(colours.len() >= 10);
        for colour in colours {
            assert!(
                ["#000000", "#e00000", "#d40000", "#a80000", "#5a0000"]
                    .iter()
                    .any(|allowed| colour.starts_with(allowed)),
                "unexpected colour {colour}"
            );
        }
        let lower = text.to_ascii_lowercase();
        for forbidden in ["rgb", "hsl", "white", "<script", "<img", "<link", "url("] {
            assert!(!lower.contains(forbidden), "unexpected {forbidden}");
        }
        for expected in [
            "<label for=\"token\">Console token</label>",
            "<input id=\"token\" name=\"token\"",
            "min-height: 44px; border: 1px solid #e00000;",
            "label { display: block; margin: 1rem 0 0.25rem; font-size: 0.875rem; font-weight: 600; color: #d40000; }",
            "input { padding: 0 12px; background: transparent;",
            "button { margin-top: 1rem; padding: 0 16px; background: #5a0000; font-weight: 560; }",
            "html, body { margin: 0; background: #000000;",
            "input:focus, button:focus { outline: 2px solid #e00000; outline-offset: 2px; }",
            "<button type=\"submit\">Pair</button>",
        ] {
            assert!(text.contains(expected), "missing {expected}");
        }
    }
}

#[tokio::test(start_paused = true)]
async fn us2_s5_pairing_cookie_opens_live_report_and_its_held_preview() {
    let sim = live::sim();
    let state = live::state(&sim, None, false)
        .with_console_token(finder::pairing::ConsoleToken::new(TOKEN).unwrap());
    let live_loop = state.live().unwrap();
    let mut reports = live_loop.subscribe();
    let phone = from(state.clone(), PHONE);
    let local = from(state, "127.0.0.1");
    let response = send(&phone, form("/pair", "token=test-console-token", ORIGIN)).await;
    assert_eq!(response.status(), 303);
    assert_eq!(response.headers()["set-cookie"], SET_COOKIE);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let response = send(
        &phone,
        get("/api/live", Some(HOST), Some(("cookie", &cookie))),
    )
    .await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    let mut stream = response.into_body().into_data_stream();
    let report = live::event(&mut reports).await;
    let chunk = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let text = std::str::from_utf8(&chunk).unwrap();
    assert!(text.lines().any(|line| line == "event: live"));
    let data = text
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap();
    assert_eq!(data, serde_json::to_string(&report).unwrap());

    let seq = report.preview_seq.unwrap();
    let path = format!("/api/live/preview.jpg?seq={seq}");
    let preview =
        answer(send(&phone, get(&path, Some(HOST), Some(("cookie", &cookie)))).await).await;
    assert_eq!(preview.0, 200);
    assert_eq!(preview.1, "image/jpeg");
    assert_eq!(preview.2, live_loop.preview(seq).unwrap().jpeg);
    assert_eq!(
        preview,
        answer(send(&local, get(&path, Some(HOST), None)).await).await
    );
    for path in ["/api/live", &path] {
        error(
            send(&phone, get(path, Some(HOST), None)).await,
            401,
            UNPAIRED,
        )
        .await;
    }
}
