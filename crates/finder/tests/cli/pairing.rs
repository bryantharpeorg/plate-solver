//! 056 US3-S3/S4: file bytes, Unix mode and exact stderr lines are the oracles.
//! Starts and the invalid-token exit have a 60 s deadline; sockets bind loopback only.
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::support::Scratch;

struct Server {
    child: Child,
    stderr: Arc<Mutex<String>>,
    reader: Option<JoinHandle<()>>,
    addresses: mpsc::Receiver<SocketAddr>,
}
impl Server {
    fn launch(folder: &Path) -> Self {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut child = Command::new(env!("CARGO_BIN_EXE_finder"))
            .arg("--db")
            .arg(repo.join("reference-solutions/cedar-solve/tetra3/data/default_database.npz"))
            .arg("--samples")
            .arg(repo.join("reference-solutions/cedar-solve/examples/data/medium_fov"))
            .args(["--listen", "127.0.0.1:0", "--state"])
            .arg(folder)
            .env("TOKIO_WORKER_THREADS", "2")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let pipe = child.stderr.take().unwrap();
        let stderr = Arc::new(Mutex::new(String::new()));
        let collected = stderr.clone();
        let (sender, addresses) = mpsc::channel();
        let reader = thread::spawn(move || {
            let mut bound = None;
            for line in BufReader::new(pipe).lines() {
                let line = line.unwrap();
                collected.lock().unwrap().push_str(&format!("{line}\n"));
                if let Some(address) = line.strip_prefix("finder console: http://") {
                    bound = Some(address.parse().unwrap());
                }
                // Wait for the next line too, so stopping immediately cannot
                // race the console's write of its listening hint.
                if line == "(to open it from a phone, run with --listen 0.0.0.0:8080)" {
                    let _ = sender.send(bound.take().expect("address precedes the hint"));
                }
            }
        });
        Self {
            child,
            stderr,
            reader: Some(reader),
            addresses,
        }
    }
    fn started(&self) -> SocketAddr {
        self.addresses
            .recv_timeout(Duration::from_secs(60))
            .expect("console must start within 60 s")
    }
    fn exited(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "invalid-token start must exit within 60 s"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
    fn stop(mut self) -> String {
        self.finish();
        self.stderr.lock().unwrap().clone()
    }
    fn finish(&mut self) {
        let _ = self.child.kill();
        self.child.wait().unwrap();
        if let Some(reader) = self.reader.take() {
            reader.join().unwrap();
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.finish();
    }
}

fn no_token_in(stderr: &str, token: &str) {
    let stderr = stderr.to_ascii_lowercase();
    let token = token.trim().to_ascii_lowercase();
    assert!(
        !stderr.contains(&token),
        "stderr must not disclose the token"
    );
    assert!(
        !stderr.contains(&token.replace('-', "")),
        "stderr must not disclose the ungrouped token"
    );
}

#[test]
fn us3_s3_first_start_creates_an_owner_only_token_and_restart_keeps_it() {
    let folder = Scratch::new("pairing-cli");
    let path = folder.0.join("console-token");
    let temporary = folder.0.join("console-token.tmp");
    fs::write(&temporary, "junk").unwrap();
    let first = Server::launch(&folder.0);
    first.started();
    let first_stderr = first.stop();
    let stored = fs::read_to_string(&path).unwrap();
    let bytes = stored.as_bytes();
    assert_eq!(bytes.len(), 15);
    for (i, byte) in bytes.iter().enumerate() {
        match i {
            4 | 9 => assert_eq!(*byte, b'-'),
            14 => assert_eq!(*byte, b'\n'),
            _ => assert!(b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(byte)),
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert!(!temporary.exists());
    assert!(first_stderr
        .lines()
        .any(|line| line == "(to open it from a phone, run with --listen 0.0.0.0:8080)"));

    let second = Server::launch(&folder.0);
    let address = second.started();
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(10)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(
        stream,
        "GET /api/session HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    let second_stderr = second.stop();
    assert!(
        fs::read_to_string(path).unwrap() == stored,
        "restart preserves the file bytes"
    );
    no_token_in(&first_stderr, &stored);
    no_token_in(&second_stderr, &stored);
}

#[cfg(unix)]
#[test]
fn us3_s4_owner_token_is_preserved_and_warned_and_an_invalid_file_stops_startup() {
    use std::os::unix::fs::PermissionsExt;

    let folder = Scratch::new("pairing-cli");
    let path = folder.0.join("console-token");
    fs::write(&path, "my-own-console-token\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let first = Server::launch(&folder.0);
    first.started();
    let stderr = first.stop();
    assert!(fs::read_to_string(&path).unwrap() == "my-own-console-token\n");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o644
    );
    assert!(stderr.lines().any(|line| line
        == format!(
            "{}: other users can read the console token; chmod 600 the file",
            path.display()
        )));
    no_token_in(&stderr, "my-own-console-token");

    fs::write(&path, "tiny7\n").unwrap();
    let mut invalid = Server::launch(&folder.0);
    let status = invalid.exited();
    let stderr = invalid.stop();
    assert!(!status.success());
    assert_eq!(status.code(), Some(1));
    assert!(stderr.lines().any(|line| line
        == format!(
            "{}: a console token is 8 to 64 letters and digits; dashes and spaces are ignored",
            path.display()
        )));
    assert!(!stderr.contains("tiny7"));
    assert!(!stderr.contains("finder console: http://"));
}
