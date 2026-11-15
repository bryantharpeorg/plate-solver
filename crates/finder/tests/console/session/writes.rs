//! Write protection: RFC 6454's scheme/host/port origin and the explicit host policy.
//! Status codes, JSON errors and unchanged sessions are exact (zero tolerance).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use finder::samples::Samples;
use finder::session::FixedClock;
use finder::{router, AppState};
use pattern_database::PatternDatabase;
use serde_json::Value;
use tower::ServiceExt;

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).unwrap())
});

fn app() -> Router {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/examples/data/medium_fov");
    router(
        AppState::shared(DB.clone(), Samples::scan(&samples).unwrap())
            .with_clock(Arc::new(FixedClock(1768528800.0))),
    )
}

fn edit(host: &str, origin: Option<&str>) -> Request<Body> {
    let mut req = Request::put("/api/session")
        .header("host", host)
        .header("content-type", "application/json");
    if let Some(origin) = origin {
        req = req.header("origin", origin);
    }
    req.body(Body::from(r#"{"target":"M42"}"#)).unwrap()
}

fn solve(host: &str, origin: Option<&str>) -> Request<Body> {
    let mut req = Request::post("/api/solve")
        .header("host", host)
        .header("content-type", "multipart/form-data; boundary=write-test");
    if let Some(origin) = origin {
        req = req.header("origin", origin);
    }
    req.body(Body::from("--write-test\r\nContent-Disposition: form-data; name=\"sample\"\r\n\r\norion2.jpg\r\n--write-test--\r\n"))
        .unwrap()
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    assert_eq!(response.headers()["content-type"], "application/json");
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn session(app: &Router) -> Value {
    let (status, body) = send(
        app,
        Request::get("/api/session").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    body
}

#[tokio::test]
async fn foreign_origins_on_both_writes_leave_target_and_pointing_unchanged() {
    let app = app();
    let mut initial = edit("127.0.0.1:8097", None);
    *initial.body_mut() = Body::from(r#"{"target":"M31"}"#);
    assert_eq!(send(&app, initial).await.0, StatusCode::OK);
    assert_eq!(
        send(&app, solve("127.0.0.1:8097", None)).await.0,
        StatusCode::OK
    );
    let before = session(&app).await;
    assert_eq!(before["target"]["messier"], "M31");
    assert!(before["pointing"].is_object());
    for request in [
        edit("127.0.0.1:8097", Some("http://elsewhere.example")),
        solve("127.0.0.1:8097", Some("http://elsewhere.example")),
    ] {
        let (status, error) = send(&app, request).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(error["error"].as_str().is_some_and(|s| !s.is_empty()));
        assert_eq!(session(&app).await, before);
    }
}

#[tokio::test]
async fn own_origin_and_absent_origin_allow_lan_localhost_and_ipv6_writes() {
    for (host, origin) in [
        ("192.168.1.20:8097", Some("http://192.168.1.20:8097")),
        ("localhost:8097", Some("http://localhost:8097")),
        ("[::1]:8097", Some("http://[::1]:8097")),
        ("LOCALHOST", Some("http://localhost:80")),
        ("surface.local:8097", None),
    ] {
        let app = app();
        let (status, body) = send(&app, edit(host, origin)).await;
        assert_eq!(status, StatusCode::OK, "{host}: {body}");
        assert_eq!(body["target"]["messier"], "M42");
        assert_eq!(session(&app).await, body);
    }
    let app = app();
    let (status, report) = send(
        &app,
        solve("192.168.1.20:8097", Some("http://192.168.1.20:8097")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["status"], "match_found");
}

#[tokio::test]
async fn plain_text_that_parses_as_json_is_415_without_an_edit() {
    let app = app();
    let before = session(&app).await;
    let mut request = edit("localhost:8097", Some("http://localhost:8097"));
    request
        .headers_mut()
        .insert("content-type", "text/plain".parse().unwrap());
    let (status, error) = send(&app, request).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert!(error["error"].is_string());
    assert_eq!(session(&app).await, before);
}

#[tokio::test]
async fn rebound_hosts_and_different_schemes_or_ports_are_forbidden_on_both_routes() {
    let app = app();
    let before = session(&app).await;
    for (host, origin) in [
        ("evil.example:8097", "http://evil.example:8097"),
        ("localhost:8097", "http://localhost:8098"),
        ("localhost:8097", "https://localhost:8097"),
        ("localhost:8097", "http://localhost.evil.example:8097"),
        ("localhost:8097", "null"),
        ("localhost:8097", "http://localhost:8097/path"),
        ("localhost:65536", "http://localhost"),
        ("localhost", "http://localhost:65536"),
        ("localhost:invalid", "http://localhost"),
        ("localhost:8097", "http://user@localhost:8097"),
    ] {
        for request in [edit(host, Some(origin)), solve(host, Some(origin))] {
            let (status, error) = send(&app, request).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{host} {origin}: {error}");
            assert!(error["error"].is_string());
            assert_eq!(session(&app).await, before);
        }
    }
}

#[tokio::test]
async fn missing_host_and_duplicate_origins_are_forbidden_before_reading_a_body() {
    let app = app();
    let mut missing = edit("localhost:8097", Some("http://localhost:8097"));
    missing.headers_mut().remove("host");
    let mut duplicate = edit("localhost:8097", Some("http://localhost:8097"));
    duplicate
        .headers_mut()
        .append("origin", "http://elsewhere.example".parse().unwrap());
    for mut request in [missing, duplicate] {
        *request.body_mut() = Body::empty();
        let (status, error) = send(&app, request).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(error["error"].is_string());
    }
}

#[test]
fn repeated_allow_host_flags_enable_only_the_named_hosts_in_the_binary() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let folder = repo
        .join("target")
        .join(format!("write-hosts-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_finder"))
        .arg("--db")
        .arg(repo.join("reference-solutions/cedar-solve/tetra3/data/default_database.npz"))
        .arg("--state")
        .arg(&folder)
        .args([
            "--listen",
            "127.0.0.1:0",
            "--allow-host",
            "surface.local",
            "--allow-host",
            "scope.local",
        ])
        .env("TOKIO_WORKER_THREADS", "2")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let pipe = child.stderr.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines() {
            let line = line.unwrap();
            if let Some(address) = line.strip_prefix("finder console: http://") {
                sender.send(address.parse::<SocketAddr>().unwrap()).unwrap();
            }
        }
    });
    let address = receiver.recv_timeout(Duration::from_secs(30));
    // Always reap the child and remove its state, including when an assertion fails.
    let result = std::panic::catch_unwind(|| {
        let address = address.expect("binary accepts --allow-host and starts");
        for (host, expected) in [
            ("surface.local", 200),
            ("scope.local", 200),
            ("evil.example", 403),
        ] {
            let mut stream = TcpStream::connect(address).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let body = r#"{"target":"M42"}"#;
            write!(stream, "PUT /api/session HTTP/1.1\r\nHost: {host}:8097\r\nOrigin: http://{host}:8097\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(
                response.starts_with(&format!("HTTP/1.1 {expected}")),
                "{response}"
            );
            let body: Value =
                serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap();
            if expected == 200 {
                assert_eq!(body["target"]["messier"], "M42");
            } else {
                assert!(body["error"].is_string());
            }
        }
    });
    let _ = child.kill();
    child.wait().unwrap();
    reader.join().unwrap();
    std::fs::remove_dir_all(folder).unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
