//! Binary oracles (026 US2): the night clock's paired flags, each refusal before
//! the console listens, and the running console on a night clock.

use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

fn command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_finder"));
    cmd.current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env("TOKIO_WORKER_THREADS", "2");
    cmd
}

/// Run the binary to a refusal: fail as soon as it starts listening, instead of
/// waiting for a process that never exits.
fn refuse(arguments: &[&str]) -> String {
    let mut child = command()
        .args(arguments)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel::<String>();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let line = line.unwrap();
            let listening = line.contains("finder console:");
            if tx.send(line).is_err() || listening {
                return;
            }
        }
    });
    let mut text = String::new();
    for line in rx {
        if line.contains("finder console:") {
            let _ = child.kill();
            let _ = child.wait();
            reader.join().unwrap();
            panic!("a refused binary started listening: {line}");
        }
        text.push_str(&line);
        text.push('\n');
    }
    reader.join().unwrap();
    let status = child.wait().unwrap();
    assert!(!status.success(), "a refused binary exited zero: {text}");
    assert!(
        !text.contains("unexpected argument"),
        "clap refused an argument it knows: {text}"
    );
    text
}

/// `--mount-sky-night` without `--covers-on` is refused before listening (US2-S1).
#[test]
fn the_night_clock_needs_the_covers_confirmed() {
    let stderr = refuse(&[
        "--mount-sky",
        "--tube-alt",
        "30",
        "--mount-sky-night",
        "2026-01-16T02:00:00Z",
    ]);
    assert!(stderr.contains("--covers-on"), "{stderr}");
}

/// `--mount-sky-night` without `--mount-sky`, and `--covers-on` alone, are each
/// refused before listening (US2-S2).
#[test]
fn the_night_flags_need_mount_sky_and_each_other() {
    for arguments in [
        vec!["--mount-sky-night", "2026-01-16T02:00:00Z", "--covers-on"],
        vec!["--covers-on"],
    ] {
        refuse(&arguments);
    }
}

struct Server {
    child: Child,
    address: SocketAddr,
    lines: Arc<Mutex<Vec<String>>>,
    reader: Option<std::thread::JoinHandle<()>>,
}
impl Server {
    fn start(mut cmd: Command) -> Self {
        let mut child = cmd
            .args(["--listen", "127.0.0.1:0"])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stderr = child.stderr.take().unwrap();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = mpsc::channel::<SocketAddr>();
        let sink = lines.clone();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let line = line.unwrap();
                sink.lock().unwrap().push(line.clone());
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
            lines,
            reader: Some(reader),
        }
    }
    /// Every standard-error line printed so far.
    fn text(&self) -> String {
        self.lines.lock().unwrap().join("\n")
    }
    fn request(&self, method: &str, route: &str, body: Value) -> Value {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let body = body.to_string();
        write!(stream,"{method} {route} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",self.address,body.len()).unwrap();
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

/// The console on a night clock reports `night_utc`, a session time within 60 s of
/// the night start, and the pretend Sun warning (US2-S3).
#[test]
fn the_console_runs_on_a_night_clock() {
    let folder = crate::support::Scratch::new("mount-sky-night");
    let mut cmd = command();
    cmd.args([
        "--state",
        folder.0.to_str().unwrap(),
        "--mount-sky",
        "--tube-alt",
        "30",
        "--mount-sky-night",
        "2026-01-16T02:00:00Z",
        "--covers-on",
    ]);
    let server = Server::start(cmd);
    let read = server.request("GET", "/api/session", json!(null));
    assert_eq!(
        read["mount_sky"]["night_utc"],
        json!("2026-01-16T02:00:00Z")
    );
    let time = read["time"].as_f64().unwrap();
    assert!(
        (1_768_528_800.0..=1_768_528_860.0).contains(&time),
        "session time {time}"
    );
    assert!(server.text().contains("pretend Sun"), "{}", server.text());
}
