//! US1-S1, S2, S5 and S9: the built node over saved camera listings and bytes.
//! Pixels and messages are exact oracles; wall-clock bounds catch silent stalls.
use scope_link::{proto, BearerToken};
use serde::Deserialize;
use std::{
    net::{SocketAddr, TcpListener},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::{Child, ChildStderr, Command},
    time::timeout,
};
use tonic::transport::Channel;

#[derive(Deserialize)]
struct Fixture {
    stream: Stream,
}
#[derive(Deserialize)]
struct Stream {
    file: String,
    metadata: String,
    frames: usize,
    frame_bytes: usize,
}

fn root(pi5: bool) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/scope-node");
    if pi5 {
        root.join("pi5")
    } else {
        root
    }
}
fn listing(pi5: bool) -> String {
    std::fs::read_to_string(root(pi5).join("list-cameras.txt")).unwrap()
}

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Node {
    child: Child,
    stderr: BufReader<ChildStderr>,
    address: SocketAddr,
    expected: Vec<Vec<u8>>,
    _directory: Scratch,
}
impl Node {
    fn spawn(name: &str, pi5_capture: bool, listing: &str, stride: usize) -> Self {
        let directory = Scratch(
            PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
                .join(format!("rpicam-node-{name}-{}", std::process::id())),
        );
        std::fs::create_dir_all(&directory.0).unwrap();
        let fixture: Fixture = serde_json::from_slice(
            &std::fs::read(root(pi5_capture).join("fixtures.json")).unwrap(),
        )
        .unwrap();
        let stream = fixture.stream;
        let bytes = std::fs::read(root(pi5_capture).join(stream.file)).unwrap();
        let metadata: Vec<serde_json::Value> = serde_json::from_slice(
            &std::fs::read(root(pi5_capture).join(stream.metadata)).unwrap(),
        )
        .unwrap();
        assert_eq!(stream.frames, 3);
        assert_eq!(bytes.len(), stream.frame_bytes * 3);
        assert_eq!(stream.frame_bytes, stride * 972 * 3 / 2);
        assert_eq!(metadata.len(), 3);
        let mut expected = Vec::new();
        let mut script = String::from(
            "#!/bin/sh\ncd -- \"$(dirname -- \"$0\")\" || exit 1\n\
             while [ $# -gt 0 ]; do\n\
              if [ \"$1\" = --metadata ]; then shift; metadata=$1; fi\n\
              shift\n\
             done\nexec 3>\"$metadata\"\nprintf '[' >&3\n",
        );
        for (i, (frame, object)) in bytes
            .chunks_exact(stream.frame_bytes)
            .zip(&metadata)
            .enumerate()
        {
            expected.push(
                frame[..stride * 972]
                    .chunks_exact(stride)
                    .flat_map(|row| row[..1296].iter().copied())
                    .collect(),
            );
            std::fs::write(directory.0.join(format!("frame-{i}.yuv")), frame).unwrap();
            std::fs::write(
                directory.0.join(format!("metadata-{i}.json")),
                serde_json::to_vec(object).unwrap(),
            )
            .unwrap();
            if i != 0 {
                script.push_str("printf ',' >&3\n");
            }
            // The real camera writes pixels before this frame's metadata. Do
            // not put either write in the background: that hides the deadlock.
            script.push_str(&format!("cat frame-{i}.yuv\ncat metadata-{i}.json >&3\n"));
        }
        script.push_str("printf ']' >&3\necho fake-camera-exit >&2\nexit 7\n");
        std::fs::write(directory.0.join("listing.txt"), listing).unwrap();
        for (name, script) in [
            ("rpicam-vid", script),
            (
                "rpicam-hello",
                "#!/bin/sh\ncd -- \"$(dirname -- \"$0\")\" || exit 1\ncat listing.txt\n".into(),
            ),
        ] {
            let path = directory.0.join(name);
            std::fs::write(&path, script).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let address = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let config = directory.0.join("config.toml");
        std::fs::write(
            &config,
            format!("listen = '{address}'\ntoken = 'test-token'\nadapter = 'rpicam'\nmode = '1296x972'\n"),
        )
        .unwrap();
        let path = std::env::join_paths(
            std::iter::once(directory.0.clone())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_scope-node"))
            .arg("--config")
            .arg(config)
            .env("PATH", path)
            .env("TMPDIR", &directory.0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stderr = BufReader::new(child.stderr.take().unwrap());
        Self {
            child,
            stderr,
            address,
            expected,
            _directory: directory,
        }
    }

    async fn wait_for_listener(&mut self) {
        timeout(Duration::from_secs(10), async {
            loop {
                assert!(
                    self.child.try_wait().unwrap().is_none(),
                    "node exited before listening"
                );
                if tokio::net::TcpStream::connect(self.address).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("node must accept a connection within 10 s");
    }

    async fn first_raw_frame(&mut self) -> proto::RawFrameReply {
        self.wait_for_listener().await;
        timeout(Duration::from_secs(10), async {
            let channel = Channel::from_shared(format!("http://{}", self.address))
                .unwrap()
                .connect()
                .await
                .unwrap();
            proto::scope_node_raw_client::ScopeNodeRawClient::with_interceptor(
                channel,
                BearerToken::new("test-token").unwrap(),
            )
            .raw_frame(proto::RawFrameRequest {})
            .await
            .unwrap()
            .into_inner()
        })
        .await
        .expect("raw frame must arrive within 10 s")
    }

    async fn kill(&mut self) {
        self.child.kill().await.unwrap();
    }
}

#[tokio::test]
async fn s1_pi5_node_returns_the_saved_luma_rows_at_stride_1408() {
    let mut node = Node::spawn("s1-pi5", true, &listing(true), 1408);
    let reply = node.first_raw_frame().await;
    node.kill().await;
    assert_eq!((reply.width, reply.height), (1296, 972));
    assert!(
        node.expected.contains(&reply.pixels),
        "pixels must match one saved frame exactly"
    );
}

#[tokio::test]
async fn s2_pi3_node_returns_the_saved_luma_rows_at_stride_1344() {
    let mut node = Node::spawn("s2-pi3", false, &listing(false), 1344);
    let reply = node.first_raw_frame().await;
    node.kill().await;
    assert_eq!((reply.width, reply.height), (1296, 972));
    assert!(
        node.expected.contains(&reply.pixels),
        "pixels must match one saved frame exactly"
    );
}

#[tokio::test]
async fn s5_unknown_pipeline_exits_one_and_names_the_handler() {
    let listing =
        listing(true).replace("PipelineHandler : rpi/pisp", "PipelineHandler : rpi/other");
    let mut node = Node::spawn("s5-unknown", true, &listing, 1408);
    let status = timeout(Duration::from_secs(5), node.child.wait())
        .await
        .expect("unknown pipeline must stop discovery within 5 s")
        .unwrap();
    let mut stderr = String::new();
    node.stderr.read_to_string(&mut stderr).await.unwrap();
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert_eq!(stderr.lines().last(), Some("scope-node: camera listing names the pipeline handler rpi/other, whose row stride the node does not know"), "{stderr}");
}

#[tokio::test]
async fn s9_each_mismatched_listing_logs_the_stride_failure() {
    for (name, pi5_capture, stride, message) in [
        ("s9-pi5-as-vc4", true, 1408, "camera capture failed: rpicam-vid: frame 1 does not fit a 1344-byte row stride at width 1296: its row padding is not 0"),
        ("s9-pi3-as-pisp", false, 1344, "camera capture failed: rpicam-vid: frame 1 does not fit a 1408-byte row stride at width 1296: its row padding is not 0"),
    ] {
        let mut node = Node::spawn(name, pi5_capture, &listing(!pi5_capture), stride);
        node.wait_for_listener().await;
        let result = timeout(Duration::from_secs(10), async {
            let mut seen = String::new();
            loop {
                let mut line = String::new();
                assert_ne!(node.stderr.read_line(&mut line).await.unwrap(), 0, "stderr ended: {seen}");
                if line.starts_with(message) {
                    return;
                }
                seen.push_str(&line);
            }
        }).await;
        node.kill().await;
        result.expect("capture must log the stride failure within 10 s after listening");
    }
}
