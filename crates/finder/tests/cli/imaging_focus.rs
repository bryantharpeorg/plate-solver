//! The built `finder` under `--simulate` turns its simulated focuser and
//! takes a focus check (033 US8): a check with no scale, no record and no
//! kept region finds the middle tile — the stars are sharpest at 0.60, 0.45
//! of the sensor — takes three noisy readings in sensor pixels near the
//! true size at a focus error of 2.0, and ends `done` with `no_record`
//! (oracle: the simulator's truth: the middle tile reads 5.6305 sensor
//! pixels at a focus error of 2.0, against 6.0386 in the tile above it, a
//! margin of 7.2%, where 2% of seeded noise either way can close at most
//! 4.1%; no scale is known, so the unit is pixels, and no record exists).
//! Real time, real sockets, generous limits.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::SocketAddr,
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

use serde_json::{json, Value};

/// A `finder` command in the repository's root, on two worker threads.
fn command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_finder"));
    cmd.current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env("TOKIO_WORKER_THREADS", "2");
    cmd
}

/// The simulated console the scenario runs: `--simulate`, `--sim-start
/// 2026-01-16T02:00:00Z`, `--state S`, `--listen` on port 0 of 127.0.0.1.
struct Server {
    child: Child,
    address: SocketAddr,
    reader: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn start(mut cmd: Command, state: &Path) -> Self {
        cmd.args([
            "--simulate",
            "--sim-start",
            "2026-01-16T02:00:00Z",
            "--listen",
            "127.0.0.1:0",
        ])
        .arg("--state")
        .arg(state);
        let mut child = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stderr = child.stderr.take().unwrap();
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let line = line.unwrap();
                if let Some(addr) = line.strip_prefix("finder console: http://") {
                    let _ = tx.send(addr.parse().unwrap());
                }
            }
        });
        let address = rx
            .recv_timeout(Duration::from_secs(30))
            .unwrap_or_else(|e| {
                let _ = child.kill();
                let _ = child.wait();
                panic!("console did not start: {e}");
            });
        Self {
            child,
            address,
            reader: Some(reader),
        }
    }

    /// One request with no `Origin`, answered as JSON. The check reads the
    /// whole status line, which the caller asserts.
    fn request(&self, method: &str, route: &str, body: Value, check: impl Fn(&str)) -> Value {
        let mut stream = std::net::TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let body = body.to_string();
        write!(
            stream,
            "{method} {route} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.address,
            body.len()
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        check(head.lines().next().unwrap());
        serde_json::from_str(body).unwrap()
    }

    /// `GET` a route with no body, as a client with no `Origin` sends it.
    fn get(&self, route: &str) -> Value {
        self.request("GET", route, json!(null), |status| {
            assert!(status.starts_with("HTTP/1.1 200"), "{status}");
        })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.reader.take().unwrap().join().unwrap();
    }
}

/// Read `GET /api/imaging` every 200 ms until `check` holds, for up to 120 s
/// of real time.
fn wait_until(server: &Server, check: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let view = server.get("/api/imaging");
        if check(&view) {
            return view;
        }
        assert!(
            Instant::now() < deadline,
            "imaging never reached the awaited state: {view}"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// US8-S3: the built `finder` under `--simulate` turns its focuser through
/// `/api/sim/focus` and a focus check finds the middle tile, takes three noisy
/// readings in sensor pixels near the true size and ends `done` with
/// `no_record` (oracle: the simulator's truth, as the module's doc names it).
#[test]
fn the_simulate_console_takes_a_focus_check_through_the_focuser() {
    let scratch = crate::support::Scratch::new("imaging-focus-cli");
    let state = scratch.0.join("state");
    let server = Server::start(command(), &state);
    wait_until(&server, |view| view["state"] == "ready");

    server.request(
        "POST",
        "/api/sim/focus",
        json!({"focus_error": 2.0}),
        |status| {
            assert!(status.starts_with("HTTP/1.1 200"), "{status}");
        },
    );
    server.request(
        "POST",
        "/api/imaging/focus",
        json!({"check": true, "exposure_ms": 100}),
        |status| {
            assert!(status.starts_with("HTTP/1.1 200"), "{status}");
        },
    );

    let view = wait_until(&server, |view| {
        view["focus"]["aid"]["state"] == json!("ended")
    });
    let aid = &view["focus"]["aid"];
    assert_eq!(aid["ended_reason"], json!("done"), "{aid}");
    assert_eq!(aid["readings"], json!(3), "{aid}");
    assert_eq!(aid["verdict"], json!("no_record"), "{aid}");
    assert_eq!(view["focus"]["unit"], json!("px"), "{view}");
    let region = &view["focus"]["region"];
    assert_eq!(region["source"], json!("found"), "{view}");
    assert_eq!(region["col"], json!(1), "{view}");
    assert_eq!(region["row"], json!(1), "{view}");

    // The reading: the true size at a focus error of 2.0 within the noise,
    // and three history values that the noise tells apart.
    let median = aid["median"]["value"].as_f64().unwrap();
    assert!((5.38..=6.00).contains(&median), "{median}");
    let history = aid["history"].as_array().unwrap();
    assert_eq!(history.len(), 3, "{aid}");
    assert!(
        history.iter().any(|value| value != &history[0]),
        "the three readings are all {history:?}"
    );
}
