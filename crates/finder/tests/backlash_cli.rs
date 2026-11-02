//! Oracle: a backlash actually measured in paused time, retained bit for bit on
//! two next-day restarts of the built binary on the same state folder.
#[path = "backlash_sim.rs"]
mod backlash_sim;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, };

use serde_json::Value;

struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!(
                "backlash-cli-{}-{}",
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
            .args(["--listen", "127.0.0.1:0", "--simulate", "--sim-start", "2026-01-17T02:00:00Z"])
            .arg("--state").arg(&folder.0)
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

#[tokio::test(start_paused = true)]
async fn measured_altitude_backlash_survives_two_next_day_binary_restarts_exactly() {
    let folder = Folder::new();
    let session = finder::session::Session::with_state_file(folder.0.join("session.json")).unwrap();
    let rig = backlash_sim::measured(session).await;
    let measured = rig.body().await["backlash"]["alt_deg"].as_f64().unwrap();
    assert!((measured-0.3).abs()*3600.0 <= 30.0);
    drop(rig);
    for _ in 0..2 {
        let server = Server::start(&folder);
        let body = server.request(None);
        assert!(body["time"].as_f64().unwrap() >= 1_768_615_200.0);
        assert_eq!(body["backlash"]["alt_deg"].as_f64(),Some(measured));
    }
}
