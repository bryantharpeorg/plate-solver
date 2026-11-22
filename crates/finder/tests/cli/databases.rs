//! US1-S1–S3. Oracles: cedar's published database properties and the CLI's
//! specified names and prefixes, compared exactly; load duration is not asserted.

use crate::support::Scratch;
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

fn cedar() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz")
}

fn command(folder: &Scratch, paths: &[&Path]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_finder"));
    for path in paths {
        command.arg("--db").arg(path);
    }
    command
        .arg("--state")
        .arg(&folder.0)
        .args(["--listen", "127.0.0.1:0"])
        .env("TOKIO_WORKER_THREADS", "2")
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

struct Server {
    child: Child,
    address: SocketAddr,
    lines: Arc<Mutex<Vec<String>>>,
    reader: Option<JoinHandle<()>>,
}

impl Server {
    fn start(folder: &Scratch, paths: &[&Path]) -> Self {
        let mut child = command(folder, paths).spawn().unwrap();
        let stderr = child.stderr.take().unwrap();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let collected = lines.clone();
        let (tx, rx) = mpsc::channel();
        let reader = thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let line = line.unwrap();
                collected.lock().unwrap().push(line.clone());
                if let Some(address) = line.strip_prefix("finder console: http://") {
                    let _ = tx.send(address.parse().unwrap());
                }
            }
        });
        let address = rx
            .recv_timeout(Duration::from_secs(30))
            .unwrap_or_else(|error| {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "console did not start: {error}: {:?}",
                    lines.lock().unwrap()
                );
            });
        Self {
            child,
            address,
            lines,
            reader: Some(reader),
        }
    }

    fn database_lines(&self) -> Vec<String> {
        self.lines
            .lock()
            .unwrap()
            .iter()
            .filter(|line| line.starts_with("database"))
            .cloned()
            .collect()
    }

    fn health(&self) -> Value {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        write!(
            stream,
            "GET /api/health HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.address
        )
        .unwrap();
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
fn repeated_db_loads_both_in_given_order_and_lists_them_in_health() {
    let folder = Scratch::new("databases-cli");
    let copy = folder.0.join("copy.npz");
    std::fs::copy(cedar(), &copy).unwrap();
    let server = Server::start(&folder, &[&copy, &cedar()]);
    let lines = server.database_lines();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines[0]
            .starts_with("database copy: 42212 stars, 1010981 patterns, FOV 10-30 deg, loaded in "),
        "{lines:?}"
    );
    assert!(
        lines[1].starts_with("database default_database: 42212 stars"),
        "{lines:?}"
    );
    let health = server.health();
    let databases = health["databases"].as_array().unwrap();
    assert_eq!(databases.len(), 2);
    assert_eq!(databases[0]["name"], "copy");
    assert_eq!(databases[1]["name"], "default_database");
    assert_eq!(databases[0]["stars"], 42212);
    assert_eq!(databases[1]["min_fov"], 10.0);
}

#[test]
fn duplicate_database_names_are_refused_at_start() {
    let folder = Scratch::new("databases-duplicate");
    let mut child = command(&folder, &[&cedar(), &cedar()]).spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("duplicate database names must fail before serving");
        }
        thread::sleep(Duration::from_millis(10));
    };
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(!status.success());
    assert!(
        stderr.contains("two databases are named default_database"),
        "{stderr}"
    );
}

#[test]
fn one_database_keeps_the_existing_start_line() {
    let folder = Scratch::new("databases-one");
    let server = Server::start(&folder, &[&cedar()]);
    let lines = server.database_lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("database: 42212 stars, 1010981 patterns, FOV 10-30 deg, loaded in "),
        "{lines:?}"
    );
}
