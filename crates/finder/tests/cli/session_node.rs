//! Binary restart oracle: exact written values, a working persisted token, and
//! Unix permission bits 0600. No timing tolerance applies to durable values.
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};
struct Server {
    child: Child,
    address: SocketAddr,
    logs: Arc<Mutex<String>>,
    reader: Option<JoinHandle<()>>,
}
impl Server {
    fn start(folder: &Path) -> Self {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut child = Command::new(env!("CARGO_BIN_EXE_finder"))
            .arg("--db")
            .arg(repo.join("reference-solutions/cedar-solve/tetra3/data/default_database.npz"))
            .arg("--state")
            .arg(folder)
            .args(["--listen", "127.0.0.1:0"])
            .env("TOKIO_WORKER_THREADS", "2")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stderr = child.stderr.take().unwrap();
        let logs = Arc::new(Mutex::new(String::new()));
        let output = logs.clone();
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let line = line.unwrap();
                output.lock().unwrap().push_str(&line);
                if let Some(addr) = line.strip_prefix("finder console: http://") {
                    let _ = tx.send(addr.parse().unwrap());
                }
            }
        });
        let address = match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(a) => a,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                reader.join().unwrap();
                panic!("{e}: {}", logs.lock().unwrap());
            }
        };
        Self {
            child,
            address,
            logs,
            reader: Some(reader),
        }
    }
    fn request(&self, edit: Option<Value>) -> Value {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let (method, body) = edit.map_or(("GET", String::new()), |v| ("PUT", v.to_string()));
        write!(stream,"{method} /api/session HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",self.address,body.len()).unwrap();
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
        assert!(!self.logs.lock().unwrap().contains("test-token"));
    }
}
struct Folder(PathBuf);
impl Drop for Folder {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn node_credentials_and_camera_settings_survive_restart_in_owner_only_state() {
    assert_restart_preserves_settings(200.0);
    assert_restart_preserves_settings(2000.0);
}

fn assert_restart_preserves_settings(exposure_ms: f64) {
    use std::os::unix::fs::PermissionsExt;
    let folder = Folder(Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/session-node-cli-{}",
        std::process::id()
    )));
    std::fs::create_dir_all(&folder.0).unwrap();
    let path = folder.0.join("session.json");
    let server = Server::start(&folder.0);
    // A prior permissive file and an interrupted temporary write must be tightened.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::write(path.with_added_extension("tmp"), "interrupted").unwrap();
    let read=server.request(Some(json!({"node":{"address":"192.168.1.40:50051","token":"test-token"},"camera_settings":{"exposure_ms":exposure_ms,"gain":4,"sigma":5}})));
    assert!(!read.to_string().contains("test-token"));
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    drop(server);
    let server = Server::start(&folder.0);
    let read = server.request(None);
    assert_eq!(read["node"]["address"], "192.168.1.40:50051");
    assert_eq!(read["node"]["token_set"], true);
    assert!(!read.to_string().contains("test-token"));
    assert_eq!(
        read["camera_settings"],
        json!({"exposure_ms":exposure_ms,"gain":4.0,"sigma":5.0})
    );
    let stored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(stored["node"]["token"], "test-token");
    assert_eq!(stored["node"]["address"], "192.168.1.40:50051");
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
