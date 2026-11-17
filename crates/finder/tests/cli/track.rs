//! Oracle: exact input values across built-binary restarts on one state folder;
//! simulator truth and mount telemetry prove Track moved before restart.
use crate::support::goto;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use finder::{model::separation, session::Session};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::Value;
use tower::ServiceExt;

struct Server {
    child: Child,
    address: SocketAddr,
    reader: Option<JoinHandle<()>>,
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
            .args([
                "--listen",
                "127.0.0.1:0",
                "--simulate",
                "--sim-start",
                "2026-01-16T02:00:00Z",
            ])
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

    fn request(&self, method: &str, path: &str, body: Value) -> Value {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let body = body.to_string();
        write!(stream, "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", self.address, body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
    }
    fn view(&self) -> Value {
        self.request("GET", "/api/session", Value::Null)
    }
    fn wait(&self, condition: impl Fn(&Value) -> bool) -> Value {
        for _ in 0..300 {
            let body = self.view();
            if condition(&body) {
                return body;
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("session condition not met: {}", self.view());
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
fn imaging_camera_survives_restart_and_null_clears_it() {
    let folder = crate::support::Scratch::new("track-cli");
    let camera = serde_json::json!({"width_mm":23.5,"height_mm":15.6,"pitch_um":3.76});
    {
        let server = Server::start(&folder);
        assert!(server.view().get("imaging_camera").is_none());
        assert_eq!(
            server.request(
                "PUT",
                "/api/session",
                serde_json::json!({"imaging_camera":camera})
            )["imaging_camera"],
            camera
        );
    }
    {
        let server = Server::start(&folder);
        assert_eq!(server.view()["imaging_camera"], camera);
        assert!(server
            .request(
                "PUT",
                "/api/session",
                serde_json::json!({"imaging_camera":null})
            )
            .get("imaging_camera")
            .is_none());
    }
    assert!(Server::start(&folder)
        .view()
        .get("imaging_camera")
        .is_none());
}
// Track on the built binary is owned by Playwright's tracking.spec.ts; this test
// owns "Track is off and nothing moves after a restart".
#[tokio::test(start_paused = true)]
async fn tracking_that_moved_the_mount_is_off_and_quiet_after_restart() {
    let folder = crate::support::Scratch::new("track-cli");
    {
        let rig = goto::Rig::with_session(
            goto::START,
            50.0,
            120.0,
            true,
            Session::with_state_file(folder.0.join("session.json")).unwrap(),
        )
        .await;
        let target = finder::targets::find("M42").unwrap();
        rig.sim.point_eyepiece_at(target.ra_deg, target.dec_deg);
        rig.sync().await;
        for _ in 0..10 {
            let p = rig.view().await["pointing"].clone();
            let (ra, dec) = (
                p["ra_deg"].as_f64().unwrap(),
                p["dec_deg"].as_f64().unwrap(),
            );
            if separation(ra, dec, target.ra_deg, target.dec_deg) < 1.0 / 60.0 {
                break;
            }
            rig.tick().await;
        }
        let request = Request::post("/api/track")
            .header("host", "localhost")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::json!({"on":true}).to_string()))
            .unwrap();
        let response = rig.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["track"]["state"], "tracking", "{body}");
        assert!(
            body.get("goto").is_none(),
            "a centred Track ran a GoTo: {body}"
        );
        let before = rig.mount.true_direction();
        // Take up the initial backlash before the tube itself starts to turn.
        for _ in 0..100 {
            rig.tick().await;
            if rig.mount.true_direction() != before {
                break;
            }
        }
        assert_ne!(rig.mount.true_direction(), before);
        drop(rig);
    }
    let stored: Value =
        serde_json::from_slice(&std::fs::read(folder.0.join("session.json")).unwrap()).unwrap();
    assert!(stored.get("track").is_none());
    let server = Server::start(&folder);
    server.wait(|v| v["mount"]["connected"] == true);
    assert!(server.view().get("track").is_none());
    let before = server.request("GET", "/api/sim/telescope", Value::Null);
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(100));
        let body = server.view();
        assert!(body.get("track").is_none());
        assert_eq!(body["mount"]["moving"], false);
    }
    let after = server.request("GET", "/api/sim/telescope", Value::Null);
    // Independent horizon transforms round at a few ulps even at rest.
    // Physical direction must be unchanged within one microarcsecond.
    let angle = finder::model::separation(
        before["eyepiece"]["az"].as_f64().unwrap(),
        before["eyepiece"]["alt"].as_f64().unwrap(),
        after["eyepiece"]["az"].as_f64().unwrap(),
        after["eyepiece"]["alt"].as_f64().unwrap(),
    );
    assert!(angle * 3600.0 < 1e-6, "restart moved {angle} degrees");
}
