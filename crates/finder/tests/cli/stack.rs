//! US8-S7: built finder, real sockets/time. Oracle: the simulator's stack
//! produces a self-contained run folder, never frames or session stack state.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

use serde_json::Value;

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
    fn start(mut cmd: Command, state: &Path, imaging_dir: Option<&Path>) -> Self {
        cmd.args(["--simulate", "--listen", "127.0.0.1:0"])
            .arg("--state")
            .arg(state);
        if let Some(dir) = imaging_dir {
            cmd.arg("--imaging-dir").arg(dir);
        }
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

    fn request(&self, method: &str, route: &str, body: &str) -> Value {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        write!(stream, "{method} {route} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", self.address, body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    /// Read `GET /api/imaging` every 200 ms for at most 20 s until `state` is
    /// `ready`, and answer the view.
    fn wait_until_ready(&self) -> Value {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let view = self.request("GET", "/api/imaging", "");
            if view["state"] == "ready" {
                return view;
            }
            assert!(
                Instant::now() < deadline,
                "imaging never became ready: {view}"
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

#[test]
fn simulate_saves_a_stack_in_the_state_folder() {
    let scratch = crate::support::Scratch::new("stack-cli");
    let state = scratch.0.join("state");
    let server = Server::start(command(), &state, None);
    server.wait_until_ready();
    let started = server.request("POST", "/api/imaging/stack", "{}");
    assert_eq!(started["stack"]["state"], "stacking");
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let view = server.request("GET", "/api/imaging", "");
        if view["stack"]["stacked"].as_u64().unwrap() >= 1 {
            break;
        }
        assert!(Instant::now() < deadline, "no sub stacked: {view}");
        std::thread::sleep(Duration::from_millis(200));
    }
    let ended = server.request("DELETE", "/api/imaging/stack", "");
    let stack = &ended["stack"];
    assert_eq!(stack["state"], "ended");
    let runs = state.join("imaging/runs");
    let folders: Vec<_> = std::fs::read_dir(&runs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(folders.len(), 1);
    assert_eq!(
        folders[0].file_name().unwrap().to_str().unwrap(),
        stack["id"].as_str().unwrap()
    );
    for name in ["stack.fits", "stack.png", "stack.json"] {
        assert!(folders[0].join(name).is_file(), "missing {name}");
    }
    let subs = std::fs::read_dir(folders[0].join("subs"))
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|ext| ext == "fits")
        })
        .count();
    assert!(subs as u64 >= stack["stacked"].as_u64().unwrap());
    fn no_files(path: &Path) {
        if path.exists() {
            for entry in std::fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                assert!(path.is_dir(), "unexpected frame {}", path.display());
                no_files(&path);
            }
        }
    }
    no_files(&state.join("imaging/frames"));
    let session = state.join("session.json");
    if session.exists() {
        assert!(!std::fs::read_to_string(session)
            .unwrap()
            .contains("\"stack\""));
    }
}
