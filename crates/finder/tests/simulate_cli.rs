//! Binary oracles: the specified UTC epoch (2 s), real-rate elapsed time (2 s),
//! no TCP connection over 5 s, and byte-exact isolation of the night's state.
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

struct Folder(PathBuf);
impl Folder {
    fn new(name: &str) -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../target/simulate-{name}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_finder"));
    cmd.current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env("TOKIO_WORKER_THREADS", "2");
    cmd
}
struct Server {
    child: Child,
    address: SocketAddr,
    reader: Option<std::thread::JoinHandle<()>>,
}
impl Server {
    fn start(mut cmd: Command) -> Self {
        let mut child = cmd
            .args(["--simulate", "--listen", "127.0.0.1:0"])
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
    fn response(&self, method: &str, route: &str, body: Value) -> String {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let body = body.to_string();
        write!(stream,"{method} {route} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",self.address,body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }
    fn request(&self, method: &str, route: &str, body: Value) -> Value {
        let response = self.response(method, route, body);
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
    }
    fn session(&self) -> Value {
        self.request("GET", "/api/session", json!(null))
    }
    fn pointing(&self) -> Value {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let read = self.session();
            if read["pointing"]["source"] == "live" {
                return read;
            }
            assert!(Instant::now() < deadline, "no simulated pointing: {read}");
            std::thread::sleep(Duration::from_millis(25));
        }
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
fn simulated_epoch_frames_and_real_rate_never_connect_to_the_stored_node() {
    let folder = Folder::new("isolation");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap().to_string();
    std::fs::write(
        folder.0.join("session.json"),
        json!({
            "site":null,"target":null,"min_alt_deg":15,
            "node":{"address":address,"token":"test-token"}
        })
        .to_string(),
    )
    .unwrap();
    let mut cmd = command();
    cmd.arg("--state")
        .arg(&folder.0)
        .args(["--sim-start", "2026-01-16T02:00:00Z"]);
    let server = Server::start(cmd);
    let first = server.session()["time"].as_f64().unwrap();
    assert!((first - 1_768_528_800.0).abs() < 2.0);
    let read = server.pointing();
    assert!(read["node"]["connected"].as_bool().unwrap());
    assert!(read["node"]["stars"].as_u64().unwrap() >= 4);
    // Pairing edits, including clearing credentials, must not divert the private link.
    server.request("PUT", "/api/session", json!({"node":null}));
    server.request(
        "PUT",
        "/api/session",
        json!({"node":{"address":address,"token":"other-test-token"}}),
    );
    let at = Instant::now();
    while at.elapsed() < Duration::from_secs(5) {
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    let next = server.session();
    assert!(
        next["pointing"]["frame_id"].as_u64().unwrap()
            > read["pointing"]["frame_id"].as_u64().unwrap()
    );
    assert!((next["time"].as_f64().unwrap() - first - 5.0).abs() < 2.0);
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn sim_start_without_simulate_is_refused_before_serving_or_writing_state() {
    let folder = Folder::new("refused");
    let output = command()
        .arg("--state")
        .arg(&folder.0)
        .args([
            "--listen",
            "127.0.0.1:0",
            "--sim-start",
            "2026-01-16T02:00:00Z",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--sim-start"), "{stderr}");
    assert!(!stderr.contains("finder console:"));
    assert!(!folder.0.join("session.json").exists());
}

#[test]
fn default_state_is_unchanged_and_simulation_keeps_its_own_aim_and_node() {
    let folder = Folder::new("xdg");
    let default = folder.0.join("finder/session.json");
    std::fs::create_dir_all(default.parent().unwrap()).unwrap();
    let bytes = b"{\n  \"site\":null,\"target\":\"M78\",\"min_alt_deg\":25\n}\n";
    std::fs::write(&default, bytes).unwrap();
    let mut cmd = command();
    cmd.env("XDG_STATE_HOME", &folder.0);
    let server = Server::start(cmd);
    let time = server.session()["time"].as_f64().unwrap();
    use finder::session::Clock;
    assert!((time - finder::session::SystemClock.now()).abs() < 2.0);
    server.request(
        "PUT",
        "/api/session",
        json!({"node":{"address":"scope.local:50051","token":"test-token"}}),
    );
    server.request("POST", "/api/sim/frames", json!({"running":false}));
    // Centre just before a preview-bearing exposure, before sidereal drift.
    std::thread::sleep(Duration::from_secs(6));
    server.request(
        "POST",
        "/api/sim/telescope",
        json!({"eyepiece_on_star":true}),
    );
    server.request("POST", "/api/sim/frames", json!({"running":true}));
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let read = server.pointing();
        // The first resumed exposure carries a preview; retry until it solves.
        let id = read["pointing"]["frame_id"].as_u64().unwrap();
        let response = server.response("POST", "/api/aim", json!({"frame_id":id,"x":706,"y":420}));
        if response.starts_with("HTTP/1.1 200") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "no calibratable preview: {response}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    drop(server);
    assert_eq!(std::fs::read(default).unwrap(), bytes);
    let stored: Value = serde_json::from_slice(
        &std::fs::read(folder.0.join("finder/simulate/session.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored["node"]["address"], "scope.local:50051");
    assert!((stored["aim"]["x"].as_f64().unwrap() - 700.0).abs() < 0.5);
    assert!((stored["aim"]["y"].as_f64().unwrap() - 420.0).abs() < 0.5);
}
