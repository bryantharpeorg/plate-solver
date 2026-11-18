//! The built `finder` solves an imaging frame under `--simulate` (032
//! US2-S6): the console answers with the simulated solver and writes the
//! solve sidecar under the state folder's `imaging` folder; the ASTAP flags
//! are checked as the scenario states. Real time, real sockets, generous
//! limits.

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

/// US2-S6: the built `finder` under `--simulate` solves a frame with the
/// simulated solver and writes its sidecar under `S/imaging`, and the ASTAP
/// flags are checked (oracle: the stated state, solver name, file and flag
/// names).
#[test]
fn the_simulate_console_solves_a_frame_with_the_simulated_solver() {
    let scratch = crate::support::Scratch::new("imaging-solve-cli");
    let state = scratch.0.join("state");
    let server = Server::start(command(), &state);
    wait_until(&server, |view| view["state"] == "ready");

    let frame = server.request("POST", "/api/imaging/frame", json!(null), |status| {
        assert!(status.starts_with("HTTP/1.1 200"), "{status}");
    });
    let id = frame["frame"]["id"].clone();
    wait_until(&server, |view| {
        view["frame"]["id"] == id && view["frame"]["state"] == "saved"
    });

    server.request("POST", "/api/imaging/solve", json!(null), |status| {
        assert!(status.starts_with("HTTP/1.1 200"), "{status}");
    });
    let view = wait_until(&server, |view| {
        view["solve"]["frame_id"] == id && view["solve"]["state"] != "solving"
    });
    assert_eq!(view["solve"]["state"], json!("solved"), "{view}");
    assert_eq!(view["solve"]["solver"], json!("simulated"), "{view}");

    let file = view["solve"]["file"].as_str().unwrap();
    let sidecar = state.join("imaging").join(file);
    assert!(
        sidecar.is_file(),
        "the solve sidecar {} is not a file under S/imaging",
        sidecar.display()
    );
}

/// US2-S6: `--astap` requires `--astap-db-name` and conflicts with
/// `--simulate` (oracle: the flag names on standard error).
#[test]
fn the_astap_flags_are_checked() {
    let out = command()
        .arg("--astap")
        .arg("/x")
        .output()
        .expect("the finder binary runs");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--astap-db-name"), "{stderr}");

    let out = command()
        .args(["--simulate", "--astap", "/x", "--astap-db-name", "d50"])
        .output()
        .expect("the finder binary runs");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--simulate") && stderr.contains("--astap"),
        "{stderr}"
    );
}
