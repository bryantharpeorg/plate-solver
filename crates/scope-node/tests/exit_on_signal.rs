//! US1-S9–S10: each signal goes only to this test's child. Exit codes and
//! stderr are independent oracles. The wall-clock exit bound is strictly 1 s;
//! the 5 s timeout only keeps a broken child from hanging the test.
use std::{
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
    task::JoinHandle,
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Node {
    child: Child,
    stderr: JoinHandle<String>,
    _directory: Scratch,
}
impl Node {
    async fn start(name: &str, missing_mount: bool) -> Self {
        let address = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let directory = Scratch(
            PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
                .join(format!("exit-on-signal-{}-{name}", std::process::id())),
        );
        std::fs::create_dir_all(&directory.0).unwrap();
        let mount = if missing_mount {
            let device = directory.0.join("missing-device");
            assert!(!device.exists());
            format!("[mount]\ndevice = '{}'\n", device.display())
        } else {
            String::new()
        };
        let config = directory.0.join("config.toml");
        std::fs::write(
            &config,
            format!("listen = '{address}'\ntoken = 'test-token'\nadapter = 'simulated'\n{mount}"),
        )
        .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_scope-node"))
            .arg("--config")
            .arg(&config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut pipe = child.stderr.take().unwrap();
        let stderr = tokio::spawn(async move {
            let mut text = String::new();
            pipe.read_to_string(&mut text).await.unwrap();
            text
        });
        let mut node = Self {
            child,
            stderr,
            _directory: directory,
        };
        node.wait_for_listener(address).await;
        node
    }
    async fn wait_for_listener(&mut self, address: SocketAddr) {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                assert!(
                    self.child.try_wait().unwrap().is_none(),
                    "node exited before listening"
                );
                if tokio::net::TcpStream::connect(address).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("scope-node must listen within 10 s");
    }
    async fn signal_and_exit(mut self, signal: &str, code: i32, last_line: &str) -> String {
        let pid = self.child.id().expect("our child is running");
        let started = Instant::now();
        let sent = Command::new("kill")
            .arg(format!("-{signal}"))
            .arg(pid.to_string())
            .status()
            .await
            .unwrap();
        assert!(sent.success(), "kill must succeed for our own child");
        let status = tokio::time::timeout(Duration::from_secs(5), self.child.wait())
            .await
            .expect("node must exit within 5 s")
            .unwrap();
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "exit took {:?}",
            started.elapsed()
        );
        let stderr = self.stderr.await.unwrap();
        assert_eq!(status.code(), Some(code), "{stderr}");
        assert_eq!(stderr.lines().last(), Some(last_line), "{stderr}");
        stderr
    }
}

#[tokio::test]
async fn s9_without_a_mount_each_signal_exits_zero_and_names_the_signal() {
    let node = Node::start("no-mount-term", false).await;
    node.signal_and_exit("TERM", 0, "scope-node: SIGTERM: no mount to stop")
        .await;
    let node = Node::start("no-mount-int", false).await;
    node.signal_and_exit("INT", 0, "scope-node: SIGINT: no mount to stop")
        .await;
}

#[tokio::test]
async fn s10_with_a_missing_device_each_signal_exits_one_without_confirmation() {
    let node = Node::start("missing-mount-term", true).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let stderr = node
        .signal_and_exit(
            "TERM",
            1,
            "scope-node: SIGTERM: the mount did not confirm the stop",
        )
        .await;
    assert!(stderr.contains("mount not connected"), "{stderr}");
    let node = Node::start("missing-mount-int", true).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let stderr = node
        .signal_and_exit(
            "INT",
            1,
            "scope-node: SIGINT: the mount did not confirm the stop",
        )
        .await;
    assert!(stderr.contains("mount not connected"), "{stderr}");
}
