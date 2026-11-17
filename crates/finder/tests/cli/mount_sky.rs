//! Binary oracles (026): the mode's flag refusals, each before the console
//! listens, and the drawn sky's isolated state folder.

use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

fn command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_finder"));
    cmd.current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
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

/// The four bad flag combinations each exit non-zero before listening (US1-S6).
#[test]
fn bad_flag_combinations_are_refused_before_listening() {
    let cases: [(Vec<&str>, Option<&str>); 4] = [
        (vec!["--mount-sky"], None),
        (
            vec!["--mount-sky", "--tube-alt", "95"],
            Some("--tube-alt requires degrees from 0 to 90"),
        ),
        (vec!["--tube-alt", "30"], None),
        (vec!["--mount-sky", "--tube-alt", "30", "--simulate"], None),
    ];
    for (arguments, message) in cases {
        let stderr = refuse(&arguments);
        if let Some(message) = message {
            assert!(stderr.contains(message), "{stderr}");
        }
    }
}

struct Server {
    child: Child,
    address: SocketAddr,
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
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        self.child.wait().unwrap();
        self.reader.take().unwrap().join().unwrap();
    }
}

/// Without `--state` the drawn sky keeps its session in `mount-sky/` (US1-S7).
#[test]
fn the_drawn_sky_keeps_its_session_in_its_own_state_folder() {
    let folder = crate::support::Scratch::new("mount-sky-state");
    let mut cmd = command();
    cmd.env("XDG_STATE_HOME", &folder.0)
        .args(["--mount-sky", "--tube-alt", "30"]);
    let server = Server::start(cmd);
    server.request("PUT", "/api/session", json!({"min_alt_deg": 20}));
    let read = server.request("GET", "/api/session", json!(null));
    assert_eq!(
        read["mount_sky"],
        json!({"tube_alt_deg": 30.0, "zero_counts": Value::Null})
    );
    assert!(folder.0.join("finder/mount-sky/session.json").exists());
    assert!(!folder.0.join("finder/session.json").exists());
}
