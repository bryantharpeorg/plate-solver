//! The built `finder` shows `--focal-length-mm` in the imaging view and
//! refuses a value that is not a number above 0 (032 US3-S6). Real time,
//! real sockets, generous limits.

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

/// The console the scenario runs: `--simulate`, `--state S`, `--listen` on
/// port 0 of 127.0.0.1, imaging frames in the state folder's `imaging`.
struct Server {
    child: Child,
    address: SocketAddr,
    reader: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn start(mut cmd: Command, state: &Path) -> Self {
        cmd.args(["--simulate", "--listen", "127.0.0.1:0"])
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

/// Read `GET /api/imaging` every 200 ms until `check` holds, for at most 30 s.
fn wait_until(server: &Server, check: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
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

/// US3-S6: the built `finder` run with `--simulate`, `--state S` and
/// `--focal-length-mm 1800` shows 1800.0 as `relation.focal_length_mm` once
/// its imaging camera is `ready` (oracle: the flag's own value).
#[test]
fn the_focal_length_flag_shows_in_the_imaging_view() {
    let scratch = crate::support::Scratch::new("imaging-hints-cli");
    let state = scratch.0.join("state");
    let mut cmd = command();
    cmd.args(["--focal-length-mm", "1800"]);
    let server = Server::start(cmd, &state);
    let view = wait_until(&server, |view| view["state"] == "ready");
    assert_eq!(
        view["relation"]["focal_length_mm"],
        json!(1800.0),
        "the focal length flag is not shown: {view}"
    );
}

/// US3-S6: `finder --focal-length-mm 0` exits with a failure and names the
/// flag on standard error (oracle: the flag's own name).
#[test]
fn a_focal_length_of_zero_is_refused() {
    let out = command()
        .args(["--focal-length-mm", "0"])
        .output()
        .expect("the finder binary runs");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--focal-length-mm"), "{stderr}");
}
