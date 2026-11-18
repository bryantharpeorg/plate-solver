//! The built `finder` saves one imaging frame under `--imaging-dir` (031
//! US2-S5): the binary takes a frame over its imaging connection, the file
//! lands in the folder the flag names and the state folder holds no `imaging`
//! folder. Real time, real sockets, generous limits.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

use serde_json::{json, Value};

struct Server {
    child: Child,
    address: SocketAddr,
    reader: Option<std::thread::JoinHandle<()>>,
}

fn command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_finder"));
    cmd.current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env("TOKIO_WORKER_THREADS", "2");
    cmd
}

impl Server {
    fn start(mut cmd: Command, state: &Path, imaging_dir: &Path) -> Self {
        cmd.args(["--simulate", "--listen", "127.0.0.1:0"])
            .arg("--state")
            .arg(state)
            .arg("--imaging-dir")
            .arg(imaging_dir);
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

    /// `GET` a route with no body, as a client with no `Origin` sends it.
    fn get(&self, route: &str) -> Value {
        self.request("GET", route, json!(null), |status| {
            assert!(status.starts_with("HTTP/1.1 200"), "{status}");
        })
    }

    /// One request with no `Origin`, answered as JSON. The check reads the
    /// whole status line, which the caller asserts.
    fn request(&self, method: &str, route: &str, body: Value, check: impl Fn(&str)) -> Value {
        let mut stream = TcpStream::connect(self.address).unwrap();
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

    /// Read `GET /api/imaging` every 200 ms until `frame.state` is `saved`, for
    /// at most 30 s.
    fn wait_until_saved(&self) -> Value {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let view = self.get("/api/imaging");
            if view["frame"]["state"] == "saved" {
                return view;
            }
            assert!(
                Instant::now() < deadline,
                "the frame was never saved: {view}"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.reader.take().unwrap().join().unwrap();
    }
}

/// Every file under a folder, recursively, as a list of paths.
fn files_under(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![folder.to_path_buf()];
    while let Some(folder) = stack.pop() {
        for entry in std::fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found
}

/// US2-S5: the built finder under `--simulate` saves one imaging frame under
/// the folder `--imaging-dir` names, and the state folder holds no `imaging`
/// folder (oracle: the simulated camera's 1236 × 822 frame).
#[test]
fn the_simulate_console_saves_one_frame_under_the_imaging_dir() {
    let scratch = crate::support::Scratch::new("imaging-frames-cli");
    let state = scratch.0.join("state");
    let imaging = scratch.0.join("frames");
    let server = Server::start(command(), &state, &imaging);
    let mut view = server.get("/api/imaging");
    let deadline = Instant::now() + Duration::from_secs(20);
    let view = loop {
        if view["state"] == "ready" {
            break view;
        }
        assert!(
            Instant::now() < deadline,
            "imaging never became ready: {view}"
        );
        std::thread::sleep(Duration::from_millis(200));
        view = server.get("/api/imaging");
    };
    assert_eq!(view["camera"]["model"], json!("simulated imaging camera"));

    server.request(
        "PUT",
        "/api/imaging/settings",
        json!({"exposure_ms":100}),
        |status| {
            assert!(status.starts_with("HTTP/1.1 200"), "{status}");
        },
    );
    server.request("POST", "/api/imaging/frame", json!(null), |status| {
        assert!(status.starts_with("HTTP/1.1 200"), "{status}");
    });
    let view = server.wait_until_saved();
    let file = view["frame"]["file"].as_str().unwrap();
    assert!(
        file.ends_with("/000001.fits") && file.starts_with("frames/"),
        "the frame's file is {file}"
    );

    let files = files_under(&imaging);
    assert_eq!(
        files.len(),
        1,
        "one file under the imaging folder: {files:?}"
    );
    let relative = Path::new(file);
    let saved = files[0]
        .strip_prefix(&imaging)
        .expect("the file is under the imaging folder");
    assert_eq!(saved, relative);
    assert_eq!(std::fs::metadata(&files[0]).unwrap().len(), 2_036_160);
    assert!(!state.join("imaging").exists());
}
