//! Binary persistence and clock contract. Oracles are the written JSON fixtures,
//! OpenNGC's M31 id and the system clock (the sole tolerance is 2 seconds).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!(
                "session-cli-{}-{}",
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
    stderr: Arc<Mutex<String>>,
    reader: Option<JoinHandle<()>>,
}
impl Server {
    fn start(folder: &Folder, explicit_state: bool) -> Self {
        Self::launch(
            folder,
            explicit_state.then_some(folder.0.as_path()),
            Some(&folder.0),
        )
    }

    fn launch(folder: &Folder, state: Option<&Path>, xdg: Option<&Path>) -> Self {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut command = Command::new(env!("CARGO_BIN_EXE_finder"));
        command
            .arg("--db")
            .arg(repo.join("reference-solutions/cedar-solve/tetra3/data/default_database.npz"))
            .arg("--samples")
            .arg(repo.join("reference-solutions/cedar-solve/examples/data/medium_fov"))
            .args(["--listen", "127.0.0.1:0"])
            .env("TOKIO_WORKER_THREADS", "2")
            .env_remove("XDG_STATE_HOME")
            .env("HOME", &folder.0)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        if let Some(state) = state {
            command.arg("--state").arg(state);
        }
        if let Some(xdg) = xdg {
            command.env("XDG_STATE_HOME", xdg);
        }
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
            stderr,
            reader: Some(reader),
        }
    }

    fn request(&self, edit: Option<Value>) -> Value {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let (method, body) = edit.map_or(("GET", String::new()), |v| ("PUT", v.to_string()));
        write!(stream, "{method} /api/session HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", self.address, body.len()).unwrap();
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

fn fresh(body: &Value) {
    assert_eq!(body["site"], Value::Null);
    assert_eq!(body["target"], Value::Null);
    assert_eq!(body["pointing"], Value::Null);
    assert_eq!(body["min_alt_deg"], 15.0);
}

#[test]
fn session_survives_a_binary_restart_without_a_pointing() {
    let folder = Folder::new();
    let server = Server::start(&folder, true);
    fresh(&server.request(None));
    let expected = server.request(Some(
        json!({"site": {"lat_deg": 40, "lon_deg": -75}, "min_alt_deg": 20, "target": "M31"}),
    ));
    assert_eq!(expected["target"]["id"], "NGC 224");
    let stored: Value =
        serde_json::from_slice(&std::fs::read(folder.0.join("session.json")).unwrap()).unwrap();
    assert_eq!(
        stored.as_object().unwrap().len(),
        3,
        "only durable fields belong in the file"
    );
    assert_eq!(
        stored,
        json!({"site": {"lat_deg": 40.0, "lon_deg": -75.0}, "min_alt_deg": 20.0, "target": "NGC 224"})
    );
    assert!(!folder.0.join("finder").exists(), "--state overrides XDG");
    assert!(!folder.0.join("session.json.tmp").exists());
    drop(server);
    // Oracle: an interrupted temporary write cannot replace the last complete file.
    std::fs::write(folder.0.join("session.json.tmp"), "interrupted write").unwrap();
    let server = Server::start(&folder, true);
    let restored = server.request(None);
    for field in ["site", "min_alt_deg", "target", "pointing"] {
        assert_eq!(restored[field], expected[field], "{field}");
    }
}

#[test]
fn corrupt_state_is_kept_beside_a_fresh_file_and_reported() {
    let folder = Folder::new();
    std::fs::write(folder.0.join("session.json"), "not json").unwrap();
    let server = Server::start(&folder, true);
    fresh(&server.request(None));
    assert_eq!(
        std::fs::read_to_string(folder.0.join("session.json.bad")).unwrap(),
        "not json"
    );
    let stderr = server.stderr.lock().unwrap().clone();
    assert!(
        stderr.contains("session.json") && stderr.contains("could not read"),
        "{stderr}"
    );
    let stored: Value =
        serde_json::from_slice(&std::fs::read(folder.0.join("session.json")).unwrap()).unwrap();
    assert_eq!(stored["site"], Value::Null);
    assert_eq!(stored["target"], Value::Null);
    assert_eq!(stored["min_alt_deg"], 15.0);
}

#[test]
fn xdg_state_home_is_the_default_folder() {
    let folder = Folder::new();
    let server = Server::start(&folder, false);
    server.request(Some(json!({"target": "M31"})));
    assert!(folder.0.join("finder/session.json").is_file());
    assert!(!folder.0.join("session.json").exists());
    drop(server);
    let server = Server::start(&folder, false);
    assert_eq!(server.request(None)["target"]["messier"], "M31");
}

#[test]
fn binary_session_time_uses_the_system_clock() {
    let folder = Folder::new();
    let server = Server::start(&folder, true);
    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let time = server.request(None)["time"].as_f64().unwrap();
    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    assert!(
        (before - 2.0..=after + 2.0).contains(&time),
        "{before} <= {time} <= {after}, tolerance 2 s"
    );
}

#[test]
fn home_is_used_when_xdg_state_home_is_unset() {
    let folder = Folder::new();
    let server = Server::launch(&folder, None, None);
    server.request(Some(json!({"target": "M42"})));
    assert!(folder.0.join(".local/state/finder/session.json").is_file());
}

#[test]
fn syntactically_valid_but_invalid_session_files_are_also_preserved() {
    for fixture in [
        json!({"site": null, "min_alt_deg": 85, "target": "M31"}),
        json!({"site": null, "min_alt_deg": 15, "target": "NGC 99999"}),
    ] {
        let folder = Folder::new();
        std::fs::write(folder.0.join("session.json"), fixture.to_string()).unwrap();
        let server = Server::start(&folder, true);
        fresh(&server.request(None));
        let preserved: Value =
            serde_json::from_slice(&std::fs::read(folder.0.join("session.json.bad")).unwrap())
                .unwrap();
        assert_eq!(preserved, fixture);
    }
}
