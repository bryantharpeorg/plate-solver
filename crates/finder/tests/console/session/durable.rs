//! Oracle: exact written fields at a fixed time, and Unix permission bits 0600.
use std::{os::unix::fs::PermissionsExt, path::PathBuf};

use finder::session::{Aim, Clock, FixedClock, Session};
use serde_json::{json, Value};

struct Folder(PathBuf);
impl Drop for Folder {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[tokio::test]
async fn durable_fields_survive_reopening_the_state_file_in_owner_only_mode() {
    let folder = Folder(
        std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("session-durable-{}", std::process::id())),
    );
    let _ = std::fs::remove_dir_all(&folder.0);
    std::fs::create_dir_all(&folder.0).unwrap();
    let path = folder.0.join("session.json");
    drop(Session::with_state_file(&path).unwrap());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    let temporary = path.with_added_extension("tmp");
    std::fs::write(&temporary, "interrupted").unwrap();
    let clock = FixedClock(1_768_528_800.0);
    let imaging_camera = json!({"width_mm": 23.5, "height_mm": 15.6, "pitch_um": 3.76});

    for exposure_ms in [200.0, 2000.0] {
        let session = Session::with_state_file(&path).unwrap();
        let settings = json!({"exposure_ms": exposure_ms, "gain": 4.0, "sigma": 5.0});
        let edited = session
            .edit(
                serde_json::from_value(json!({
                    "site": {"lat_deg": 40.0, "lon_deg": -75.0},
                    "min_alt_deg": 30.0,
                    "target": "M42",
                    "imaging_camera": imaging_camera,
                    "node": {"address": "192.168.1.40:50051", "token": "test-token"},
                    "camera_settings": settings,
                }))
                .unwrap(),
                clock.now(),
            )
            .unwrap();
        assert!(!serde_json::to_string(&edited)
            .unwrap()
            .contains("test-token"));
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(
            !temporary.exists(),
            "stale temporary state survived a write"
        );
        let written = session
            .set_aim(Some(Aim { x: 700.0, y: 420.0 }), clock.now())
            .unwrap();
        drop(session);

        let session = Session::with_state_file(&path).unwrap();
        let read = serde_json::to_value(session.snapshot(clock.now())).unwrap();
        assert_eq!(read["site"], json!({"lat_deg": 40.0, "lon_deg": -75.0}));
        assert_eq!(read["min_alt_deg"], 30.0);
        assert_eq!(read["target"]["id"], "NGC 1976");
        assert_eq!(read["target"]["messier"], "M42");
        assert_eq!(read["node"]["address"], "192.168.1.40:50051");
        assert_eq!(read["node"]["token_set"], true);
        assert_eq!(read["camera_settings"], settings);
        assert_eq!(read["imaging_camera"], imaging_camera);
        assert_eq!(read["aim"], json!({"x": 700.0, "y": 420.0}));
        assert!(!read.to_string().contains("test-token"));
        assert_eq!(read, serde_json::to_value(written).unwrap());
        let stored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(stored["node"]["token"], "test-token");
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
