//! US7 persistence oracle: exact supplied camera values and the simulator's
//! axis positions across restarts of the built binary on one --state folder.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{json, Value};

struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!(
                "track-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
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

struct Server {
    child: Child,
    address: SocketAddr,
    reader: Option<JoinHandle<()>>,
}
impl Server {
    fn start(folder: &Folder) -> Self {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut command = Command::new(env!("CARGO_BIN_EXE_finder"));
        command
            .arg("--db")
            .arg(repo.join("reference-solutions/cedar-solve/tetra3/data/default_database.npz"))
            .arg("--samples")
            .arg(repo.join("reference-solutions/cedar-solve/examples/data/medium_fov"))
            .args([
                "--listen",
                "127.0.0.1:0",
                "--simulate",
                "--sim-start",
                "2026-01-16T02:00:00Z",
            ])
            .arg("--state")
            .arg(&folder.0)
            .env("TOKIO_WORKER_THREADS", "2")
            .env_remove("XDG_STATE_HOME")
            .env("HOME", &folder.0)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();
        let pipe = child.stderr.take().unwrap();
        let stderr = Arc::new(Mutex::new(String::new()));
        let collected = stderr.clone();
        let (sender, receiver) = mpsc::channel();
        let reader = thread::spawn(move || {
            for line in BufReader::new(pipe).lines() {
                let line = line.unwrap();
                collected.lock().unwrap().push_str(&format!("{line}\n"));
                if let Some(address) = line.strip_prefix("finder console: http://") {
                    sender.send(address.parse::<SocketAddr>().unwrap()).unwrap();
                }
            }
        });
        let address = match receiver.recv_timeout(Duration::from_secs(30)) {
            Ok(address) => address,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                reader.join().unwrap();
                panic!("server did not start: {error}: {}", stderr.lock().unwrap());
            }
        };
        Self {
            child,
            address,
            reader: Some(reader),
        }
    }

    fn request(&self, method: &str, path: &str, edit: Option<Value>) -> Value {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let body = edit.map_or_else(String::new, |v| v.to_string());
        write!(stream, "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", self.address, body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
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
fn imaging_camera_survives_restart_exactly_and_explicit_null_clears_it() {
    let folder = Folder::new();
    let expected = json!({"width_mm":23.5,"height_mm":15.6,"pitch_um":3.76});
    {
        let server = Server::start(&folder);
        assert!(server
            .request("GET", "/api/session", None)
            .get("imaging_camera")
            .is_none());
        let body = server.request(
            "PUT",
            "/api/session",
            Some(json!({"imaging_camera":expected})),
        );
        assert_eq!(body["imaging_camera"], expected);
    }
    {
        let server = Server::start(&folder);
        assert_eq!(
            server.request("GET", "/api/session", None)["imaging_camera"],
            expected
        );
        let body = server.request("PUT", "/api/session", Some(json!({"imaging_camera":null})));
        assert!(body.get("imaging_camera").is_none());
    }
    let server = Server::start(&folder);
    assert!(server
        .request("GET", "/api/session", None)
        .get("imaging_camera")
        .is_none());
}

fn wait_for(server: &Server, mut condition: impl FnMut(&Value) -> bool) -> Value {
    let start = std::time::Instant::now();
    loop {
        let body = server.request("GET", "/api/session", None);
        if condition(&body) {
            return body;
        }
        assert!(
            start.elapsed() < Duration::from_secs(90),
            "timed out: {body}"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn tracking_that_moved_before_restart_is_off_and_axes_stay_still_after_restart() {
    let folder = Folder::new();
    // Saved camera calibration is an input fixture, not a synthetic sync.
    std::fs::write(
        folder.0.join("session.json"),
        json!({
            "site":{"lat_deg":40,"lon_deg":-75},"min_alt_deg":15,"target":"M42",
            "aim":{"x":700,"y":420},"camera_settings":{"exposure_ms":200,"gain":1,"sigma":8}
        })
        .to_string(),
    )
    .unwrap();
    {
        let server = Server::start(&folder);
        // Start at M42, using the simulator's independent
        // horizon transform. This exercises tracking without a long real slew.
        let m42 = finder::targets::find("M42").unwrap();
        let body = server.request("GET", "/api/session", None);
        let now = body["time"].as_f64().unwrap();
        let (alt, az) = simulator::sky::horizontal(m42.vector(), simulator::Site::default(), now);
        server.request(
            "POST",
            "/api/sim/telescope",
            Some(json!({"alt_deg":alt,"az_deg":az})),
        );
        wait_for(&server, |v| {
            v["mount"]["connected"] == true && v["model"]["syncs"].as_u64().is_some_and(|n| n > 0)
        });
        server.request("POST", "/api/track", Some(json!({"on":true})));
        wait_for(&server, |v| {
            v["track"]["state"] == "tracking" && v["mount"]["moving"] == true
        });
        let before = server.request("GET", "/api/sim/telescope", None);
        thread::sleep(Duration::from_millis(1200));
        let after = server.request("GET", "/api/sim/telescope", None);
        assert_ne!(
            after["eyepiece"]["az"], before["eyepiece"]["az"],
            "accepted tracking must move an axis"
        );
        let stored: Value =
            serde_json::from_slice(&std::fs::read(folder.0.join("session.json")).unwrap()).unwrap();
        assert!(stored.get("track").is_none());
    }
    let server = Server::start(&folder);
    let body = wait_for(&server, |v| v["mount"]["connected"] == true);
    assert!(
        body.get("track").is_none(),
        "tracking must be off on every binary start"
    );
    assert_eq!(body["mount"]["moving"], false);
    let before = server.request("GET", "/api/sim/telescope", None);
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(100));
        let body = server.request("GET", "/api/session", None);
        assert_eq!(body["mount"]["moving"], false);
        assert!(body.get("track").is_none());
        let after = server.request("GET", "/api/sim/telescope", None);
        // The simulator converts through celestial coordinates for this HTTP
        // truth hook. Allow 1 microarcsecond for that floating-point round trip.
        let distance = finder::model::separation(
            before["eyepiece"]["az"].as_f64().unwrap(),
            before["eyepiece"]["alt"].as_f64().unwrap(),
            after["eyepiece"]["az"].as_f64().unwrap(),
            after["eyepiece"]["alt"].as_f64().unwrap(),
        );
        assert!(distance * 3600.0 < 1e-6);
    }
}
