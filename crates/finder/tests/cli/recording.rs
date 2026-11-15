//! Oracle: the built console's explicit state folder, session and on-disk files.
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

struct Console {
    child: Child,
    root: PathBuf,
}
impl Drop for Console {
    fn drop(&mut self) {
        let _ = self.child.kill();
        self.child.wait().unwrap();
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
fn request(address: SocketAddr, method: &str, route: &str, body: Value) -> Value {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let body = body.to_string();
    write!(stream, "{method} {route} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
}
#[test]
fn simulated_binary_records_in_its_explicit_state_folder() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/recording-binary-{}",
        std::process::id()
    ));
    assert!(!root.exists());
    let child = Command::new(env!("CARGO_BIN_EXE_finder"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env("TOKIO_WORKER_THREADS", "2")
        .args(["--simulate", "--listen", "127.0.0.1:0", "--state"])
        .arg(&root)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut console = Console { child, root };
    let stderr = console.child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let line = line.unwrap();
            if let Some(address) = line.strip_prefix("finder console: http://") {
                let _ = tx.send(address.parse::<SocketAddr>().unwrap());
            }
        }
    });
    let address = rx.recv_timeout(Duration::from_secs(30)).unwrap();
    request(address, "POST", "/api/record", json!({"on":true}));
    let session = request(address, "GET", "/api/session", Value::Null);
    assert_eq!(session["recording"]["on"], true);
    let folders: Vec<_> = std::fs::read_dir(console.root.join("recordings"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(folders.len(), 1);
    assert_eq!(
        folders[0].file_name().unwrap().to_str().unwrap(),
        session["recording"]["folder"].as_str().unwrap()
    );
    assert!(folders[0].join("recording.json").is_file());
    request(address, "POST", "/api/record", json!({"on":false}));
    let persisted: Value =
        serde_json::from_slice(&std::fs::read(console.root.join("session.json")).unwrap()).unwrap();
    assert!(persisted.get("recording").is_none());
    drop(console);
    reader.join().unwrap();
}
