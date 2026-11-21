//! US8-S3: the built console wires its own run parts under --simulate.
//! Oracle: the exact no-target refusal, after the imaging camera is ready.
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

impl Server {
    fn start(state: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_finder"))
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .env("TOKIO_WORKER_THREADS", "2")
            .args(["--simulate", "--listen", "127.0.0.1:0", "--state"])
            .arg(state)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stderr = child.stderr.take().unwrap();
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let line = line.unwrap();
                if let Some(address) = line.strip_prefix("finder console: http://") {
                    let _ = tx.send(address.parse().unwrap());
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

    fn request(&self, method: &str, path: &str, body: &str) -> (u16, Value) {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        write!(stream, "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", self.address, body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (header, body) = response.split_once("\r\n\r\n").unwrap();
        let status = header.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, serde_json::from_str(body).unwrap())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        self.child.wait().unwrap();
        self.reader.take().unwrap().join().unwrap();
    }
}

#[test]
fn simulated_binary_has_imaging_runs_and_refuses_a_missing_target() {
    let scratch = crate::support::Scratch::new("imaging-run-cli");
    let server = Server::start(&scratch.0);
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let (status, view) = server.request("GET", "/api/imaging", "");
        assert_eq!(status, 200, "{view}");
        if view["state"] == "ready" {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "imaging never became ready: {view}"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    let (status, view) = server.request("POST", "/api/imaging/run", "{}");
    assert_eq!(status, 409, "{view}");
    assert_eq!(view["error"], "no target is selected");
}
