//! The chart rotation's restart contract. Oracles: the written `session.json`
//! and the angle it is read back at (US1-S3).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

struct Server {
    child: Child,
    address: SocketAddr,
    reader: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn start(folder: &crate::support::Scratch) -> Self {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut command = Command::new(env!("CARGO_BIN_EXE_finder"));
        command
            .arg("--db")
            .arg(repo.join("reference-solutions/cedar-solve/tetra3/data/default_database.npz"))
            .arg("--samples")
            .arg(repo.join("reference-solutions/cedar-solve/examples/data/medium_fov"))
            .args(["--listen", "127.0.0.1:0"])
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

fn stored(folder: &crate::support::Scratch) -> Value {
    serde_json::from_slice(&std::fs::read(folder.0.join("session.json")).unwrap()).unwrap()
}

#[test]
fn rotation_survives_a_restart_and_null_clears_it_from_read_and_file() {
    let folder = crate::support::Scratch::new("session-rotation");
    let server = Server::start(&folder);
    let edited = server.request(Some(json!({"chart_rotation_deg": 90})));
    assert_eq!(edited["chart_rotation_deg"], 90.0);
    assert_eq!(stored(&folder)["chart_rotation_deg"], 90.0);
    drop(server);
    // Oracle: the same state folder, started again, keeps the angle.
    let server = Server::start(&folder);
    let restored = server.request(None);
    assert_eq!(restored["chart_rotation_deg"], 90.0);
    assert_eq!(stored(&folder)["chart_rotation_deg"], 90.0);
    let cleared = server.request(Some(json!({"chart_rotation_deg": null})));
    assert!(
        !cleared
            .as_object()
            .unwrap()
            .contains_key("chart_rotation_deg"),
        "{cleared}"
    );
    let after: Value = stored(&folder);
    assert!(
        !after
            .as_object()
            .unwrap()
            .contains_key("chart_rotation_deg"),
        "{after}"
    );
    drop(server);
    // Oracle: a hand-edited stored value is checked and normalised as a PUT is.
    std::fs::write(
        folder.0.join("session.json"),
        json!({"site": null, "min_alt_deg": 15, "target": null, "chart_rotation_deg": 450})
            .to_string(),
    )
    .unwrap();
    let server = Server::start(&folder);
    assert_eq!(server.request(None)["chart_rotation_deg"], 90.0);
}
