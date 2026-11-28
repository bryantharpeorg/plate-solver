//! Helpers shared by finder's integration-test binaries. Nothing under this directory
//! is a test: a test function is compiled into exactly one binary.
pub mod goto;
pub mod live;
pub mod mcp;
pub mod stop;
pub mod tracker;

/// A folder for one test's files, under the target directory cargo names, removed
/// when dropped. The name, process id and a counter keep it unique under nextest and
/// under `cargo test`.
pub struct Scratch(pub std::path::PathBuf);
impl Scratch {
    pub fn new(name: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Router-only console pairing rig. It lives here because 027's layout test pins
/// the support folder's files; all expectations come from 056's scenarios.
pub mod pairing {
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    use axum::body::{to_bytes, Body};
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::{Request, StatusCode};
    use axum::response::Response;
    use axum::Router;
    use finder::pairing::ConsoleToken;
    use finder::session::Clock;
    use finder::{router, AppState};
    use serde_json::Value;
    use tower::ServiceExt;

    pub const TOKEN: &str = "test-console-token";
    pub const T: f64 = 1_768_528_800.0;
    pub const PHONE: &str = "192.168.1.30";
    pub const OTHER_PHONE: &str = "192.168.1.31";
    pub const HOST: &str = "192.168.1.20:8080";
    pub const ORIGIN: &str = "http://192.168.1.20:8080";
    pub const COOKIE: &str = "console_token=TESTCONSOLETOKEN";
    pub const UNPAIRED: &str =
        "this device is not paired: open the console's page and enter the console token";
    pub const WRONG: &str = "that is not the console token";
    pub const LIMITED: &str = "too many wrong console tokens from this device: wait a minute";
    pub const BAD_HOST: &str = "open the console at an IP address, localhost or an allowed host";
    pub const READS: &[&str] = &[
        "/api/session",
        "/api/health",
        "/api/live",
        "/api/live/preview.jpg?seq=1",
        "/api/imaging/frames/1/preview.jpg",
        "/",
    ];

    pub struct StepClock(Mutex<f64>);
    impl StepClock {
        pub fn set(&self, now: f64) {
            *self.0.lock().unwrap() = now;
        }
    }
    impl Clock for StepClock {
        fn now(&self) -> f64 {
            *self.0.lock().unwrap()
        }
    }
    pub fn clock() -> Arc<StepClock> {
        Arc::new(StepClock(Mutex::new(T)))
    }
    pub fn gated(clock: Arc<StepClock>) -> AppState {
        super::mcp::state()
            .with_clock(clock)
            .with_console_token(ConsoleToken::new(TOKEN).unwrap())
    }
    pub fn from(state: AppState, peer: &str) -> Router {
        router(state).layer(MockConnectInfo(SocketAddr::new(
            peer.parse().unwrap(),
            12345,
        )))
    }
    pub fn get(path: &str, host: Option<&str>, header: Option<(&str, &str)>) -> Request<Body> {
        let mut request = Request::get(path);
        if let Some(host) = host {
            request = request.header("host", host);
        }
        if let Some((name, value)) = header {
            request = request.header(name, value);
        }
        request.body(Body::empty()).unwrap()
    }
    pub async fn send(app: &Router, request: Request<Body>) -> Response {
        app.clone().oneshot(request).await.unwrap()
    }
    pub async fn json_of(response: Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
    pub async fn answer(response: Response) -> (StatusCode, String, Vec<u8>) {
        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .map_or("", |value| value.to_str().unwrap())
            .to_owned();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, content_type, body.to_vec())
    }
}
