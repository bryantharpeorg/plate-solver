//! The built `finder`'s imaging view over `--simulate` (031 US1-S7): the binary
//! reaches `ready` with the simulated imaging camera, takes `--imaging-dir`, and
//! names the flag in `--help`. Real time, real sockets, generous limits.

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

    /// `GET` a route with no body, as a client with no `Origin` sends it.
    fn get(&self, route: &str) -> Value {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        write!(
            stream,
            "GET {route} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.address
        )
        .unwrap();
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
            let view = self.get("/api/imaging");
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

/// US1-S7: the built finder under `--simulate` reaches `ready` with the simulated
/// imaging camera, with and without `--imaging-dir`, and names the flag.
#[test]
fn the_simulate_console_sees_the_simulated_imaging_camera() {
    let scratch = crate::support::Scratch::new("imaging-cli");
    let state = scratch.0.join("state");
    let server = Server::start(command(), &state, None);
    let view = server.wait_until_ready();
    assert_eq!(view["camera"]["model"], "simulated imaging camera");
    assert_eq!(view["settings"]["gain"], serde_json::json!(120.0));

    let imaging = scratch.0.join("frames");
    let server = Server::start(command(), &state, Some(&imaging));
    let view = server.wait_until_ready();
    assert_eq!(view["camera"]["model"], "simulated imaging camera");

    let help = command()
        .arg("--help")
        .output()
        .expect("finder --help runs");
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(
        help.contains("--imaging-dir"),
        "finder --help does not name --imaging-dir:\n{help}"
    );
}
