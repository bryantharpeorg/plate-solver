use scope_node::{
    capture::CaptureLoop,
    rpicam::{listing, metadata::MetadataReader, yuv::YuvReader, RpicamCamera},
    Camera, Settings,
};
use serde::Deserialize;
use std::{path::PathBuf, time::Duration};

#[cfg(unix)]
struct Scratch(PathBuf);
#[cfg(unix)]
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Deserialize)]
struct Fixture {
    camera: ExpectedCamera,
    stride_alignment: usize,
    stream: Stream,
    listing: Listing,
}
#[derive(Deserialize)]
struct ExpectedCamera {
    model: String,
    width: u32,
    height: u32,
    modes: Vec<Size>,
    exposure_time_us: Range,
    analogue_gain: Range,
}
#[derive(Deserialize)]
struct Range {
    min: f64,
    max: f64,
}
#[derive(Deserialize, Debug, PartialEq)]
struct Size {
    width: u32,
    height: u32,
}
#[derive(Deserialize)]
struct Listing {
    file: String,
}
#[derive(Deserialize)]
struct Stream {
    file: String,
    metadata: String,
    width: u32,
    height: u32,
    stride: usize,
    frames: usize,
    frame_bytes: usize,
    sensor_timestamps_ns: Vec<u64>,
    exposure_times_us: Vec<u64>,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/scope-node")
}
fn fixture() -> Fixture {
    serde_json::from_slice(&std::fs::read(root().join("fixtures.json")).unwrap()).unwrap()
}

#[test]
fn listing_reports_every_saved_sensor_value() {
    let f = fixture();
    let cameras =
        listing::parse(&std::fs::read_to_string(root().join(f.listing.file)).unwrap()).unwrap();
    let info = &cameras[0];
    assert_eq!(info.model, f.camera.model);
    assert_eq!((info.width, info.height), (f.camera.width, f.camera.height));
    assert_eq!(info.exposure_min_ms, f.camera.exposure_time_us.min / 1000.0);
    assert_eq!(info.exposure_max_ms, f.camera.exposure_time_us.max / 1000.0);
    assert_eq!(info.gain_min, f.camera.analogue_gain.min);
    assert_eq!(info.gain_max, f.camera.analogue_gain.max);
    assert_eq!(
        info.modes
            .iter()
            .map(|m| Size {
                width: m.width,
                height: m.height
            })
            .collect::<Vec<_>>(),
        f.camera.modes
    );
}

#[tokio::test]
async fn padded_stream_and_metadata_match_saved_oracles_byte_for_byte() {
    let f = fixture();
    let s = f.stream;
    assert_eq!(scope_node::rpicam::yuv::stride(s.width), s.stride);
    assert_eq!(s.stride % f.stride_alignment, 0);
    assert!(s.stride > s.width as usize, "fixture must exercise padding");
    let bytes = std::fs::read(root().join(s.file)).unwrap();
    assert_eq!(bytes.len(), s.frames * s.frame_bytes);
    let metadata = std::fs::read(root().join(s.metadata)).unwrap();
    let mut yuv = YuvReader::new(bytes.as_slice(), s.width, s.height, s.stride).unwrap();
    let mut meta = MetadataReader::new(metadata.as_slice());
    for i in 0..s.frames {
        let frame = meta.next().await.unwrap().unwrap().frame(
            yuv.next().await.unwrap().unwrap(),
            s.width,
            s.height,
        );
        let saved = &bytes[i * s.frame_bytes..(i + 1) * s.frame_bytes];
        let expected = saved[..s.stride * s.height as usize]
            .chunks_exact(s.stride)
            .flat_map(|row| row[..s.width as usize].iter().copied())
            .collect::<Vec<_>>();
        assert_eq!(frame.pixels, expected);
        let start = s.sensor_timestamps_ns[i] as f64 / 1e9;
        let end = start + s.exposure_times_us[i] as f64 / 1e6;
        // Unit conversions from the operator's integers; exact f64 equality.
        assert_eq!(frame.start_s, start);
        assert_eq!(frame.end_s, end);
        assert_eq!(frame.exposure_ms, s.exposure_times_us[i] as f64 / 1000.0);
    }
    assert!(yuv.next().await.unwrap().is_none());
    assert!(meta.next().await.unwrap().is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn subprocess_exit_reports_stderr_and_capture_restarts_without_restarting_service() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture();
    let dir = Scratch(
        PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("rpicam-test-{}", std::process::id())),
    );
    std::fs::create_dir_all(&dir.0).unwrap();
    let script = dir.0.join("rpicam-vid");
    // The fake accepts precisely the adapter's metadata destination. PATH is set
    // on the child command, avoiding process-global environment races in tests.
    std::fs::write(&script, format!("#!/bin/sh\nwhile [ $# -gt 0 ]; do\n if [ \"$1\" = --metadata ]; then shift; metadata=$1; fi\n shift\ndone\ncat '{}' > \"$metadata\" &\ncat '{}'\nwait\necho fixture-camera-exit >&2\nexit 7\n", root().join(&f.stream.metadata).display(), root().join(&f.stream.file).display())).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut info = listing::parse(&std::fs::read_to_string(root().join(&f.listing.file)).unwrap())
        .unwrap()
        .remove(0);
    info.active_mode = Some(
        *info
            .modes
            .iter()
            .find(|m| m.width == f.stream.width && m.height == f.stream.height)
            .unwrap(),
    );
    let path = std::env::join_paths(
        std::iter::once(dir.0.clone())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let mut camera = RpicamCamera::new(info).unwrap().with_path(path);
    camera
        .apply(&Settings {
            exposure_ms: Some(f.stream.exposure_times_us[0] as f64 / 1000.0),
            gain: Some(f.camera.analogue_gain.min),
            ..Settings::default()
        })
        .unwrap();
    for _ in 0..f.stream.frames {
        camera.next_frame().await.unwrap();
    }
    let error = camera.next_frame().await.unwrap_err().to_string();
    assert!(error.contains("7"), "{error}");
    assert!(error.contains("fixture-camera-exit"), "{error}");
    let pid = std::process::id();
    let mut capture = CaptureLoop::new(camera, Settings::default()).unwrap();
    for _ in 0..f.stream.frames {
        capture.next().await.unwrap();
    }
    let error = capture.next().await.unwrap_err().to_string();
    assert!(error.contains("fixture-camera-exit"));
    let before = tokio::time::Instant::now();
    let next = tokio::time::timeout(Duration::from_secs(5), capture.next())
        .await
        .unwrap()
        .unwrap();
    assert!(before.elapsed() >= Duration::from_secs(1));
    assert_eq!(next.id, f.stream.frames as u64 + 1);
    assert_eq!(capture.restarts(), 1);
    assert_eq!(std::process::id(), pid);
    // A control change discards unread frames and reaps the old camera before
    // opening another one. The replay starts again at the saved first timestamp.
    capture
        .apply(Settings {
            exposure_ms: Some(f.stream.exposure_times_us[0] as f64 / 500.0),
            ..Settings::default()
        })
        .unwrap();
    let reopened = capture.next().await.unwrap();
    assert_eq!(
        reopened.frame.start_s,
        f.stream.sensor_timestamps_ns[0] as f64 / 1e9
    );
    assert_eq!(reopened.id, next.id + 1);
    drop(capture);
}
