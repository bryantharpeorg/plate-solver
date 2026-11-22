//! US1-S3, S4 and S6–S8: measured strides, saved listings, and exact stream bytes.
use scope_node::{
    rpicam::{
        listing,
        yuv::{self, Isp, YuvReader},
        RpicamCamera,
    },
    Camera, GrayFrame,
};
use serde::Deserialize;
use std::{io, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};
use tokio::time::timeout;

fn root(pi5: bool) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/scope-node");
    if pi5 {
        root.join("pi5")
    } else {
        root
    }
}
fn saved_listing(pi5: bool) -> String {
    std::fs::read_to_string(root(pi5).join("list-cameras.txt")).unwrap()
}

#[derive(Deserialize)]
struct Fixture {
    measured_strides: Vec<MeasuredStride>,
    stream: SavedStream,
}
#[derive(Deserialize)]
struct MeasuredStride {
    width: u32,
    stride: usize,
}
#[derive(Deserialize)]
struct SavedStream {
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
fn fixture() -> Fixture {
    serde_json::from_slice(&std::fs::read(root(true).join("fixtures.json")).unwrap()).unwrap()
}

struct Stream {
    width: u32,
    height: u32,
    frames: Vec<Vec<u8>>,
    gray: Vec<Vec<u8>>,
}
impl Stream {
    fn saved(s: &SavedStream) -> Self {
        let bytes = std::fs::read(root(true).join(&s.file)).unwrap();
        assert_eq!(
            (s.width, s.height, s.stride, s.frames),
            (1296, 972, 1408, 3)
        );
        assert_eq!(s.frame_bytes, 2_052_864);
        assert_eq!(bytes.len(), s.frames * s.frame_bytes);
        let frames: Vec<Vec<u8>> = bytes
            .chunks_exact(s.frame_bytes)
            .map(<[u8]>::to_vec)
            .collect();
        let gray = frames
            .iter()
            .map(|frame| {
                frame[..1408 * 972]
                    .as_chunks::<1408>()
                    .0
                    .iter()
                    .flat_map(|row| row[..1296].iter().copied())
                    .collect()
            })
            .collect();
        Self {
            width: s.width,
            height: s.height,
            frames,
            gray,
        }
    }

    // The scenario's byte recipe, independent of the adapter and its stride rule.
    fn synthetic(width: u32, height: u32, stride: usize, dark: bool) -> Self {
        let (w, h) = (width as usize, height as usize);
        let mut frames = Vec::new();
        let mut gray = Vec::new();
        for k in 0..3 {
            let pixels: Vec<u8> = (0..h)
                .flat_map(|r| {
                    (0..w).map(move |c| {
                        if dark {
                            0
                        } else {
                            (1 + (31 * r + 7 * c + 13 * k) % 250) as u8
                        }
                    })
                })
                .collect();
            let mut frame = vec![0; stride * h * 3 / 2];
            for (row, source) in frame[..stride * h]
                .chunks_exact_mut(stride)
                .zip(pixels.chunks_exact(w))
            {
                row[..w].copy_from_slice(source);
            }
            for row in frame[stride * h..].chunks_exact_mut(stride / 2) {
                row[..w / 2].fill(128);
            }
            frames.push(frame);
            gray.push(pixels);
        }
        Self {
            width,
            height,
            frames,
            gray,
        }
    }
}

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Rig {
    camera: RpicamCamera,
    _directory: Scratch,
}
impl Rig {
    fn new(name: &str, listing: &str, stream: &Stream) -> Self {
        let directory = Scratch(
            PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
                .join(format!("rpicam-stride-{name}-{}", std::process::id())),
        );
        std::fs::create_dir_all(&directory.0).unwrap();
        let s = fixture().stream;
        let metadata: Vec<serde_json::Value> =
            serde_json::from_slice(&std::fs::read(root(true).join(s.metadata)).unwrap()).unwrap();
        assert_eq!(stream.frames.len(), 3);
        assert_eq!(metadata.len(), 3);
        let mut script = String::from(
            "#!/bin/sh\ncd -- \"$(dirname -- \"$0\")\" || exit 1\n\
             while [ $# -gt 0 ]; do\n\
              if [ \"$1\" = --metadata ]; then shift; metadata=$1; fi\n\
              shift\n\
             done\nexec 3>\"$metadata\"\nprintf '[' >&3\n",
        );
        for (i, (frame, object)) in stream.frames.iter().zip(&metadata).enumerate() {
            std::fs::write(directory.0.join(format!("frame-{i}.yuv")), frame).unwrap();
            std::fs::write(
                directory.0.join(format!("metadata-{i}.json")),
                serde_json::to_vec(object).unwrap(),
            )
            .unwrap();
            if i != 0 {
                script.push_str("printf ',' >&3\n");
            }
            // No concurrent metadata writer: pixels must finish first, as on the Pi.
            script.push_str(&format!("cat frame-{i}.yuv\ncat metadata-{i}.json >&3\n"));
        }
        script.push_str("printf ']' >&3\necho fake-camera-exit >&2\nexit 7\n");
        let executable = directory.0.join("rpicam-vid");
        std::fs::write(&executable, script).unwrap();
        std::fs::set_permissions(executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut info = listing::parse(listing).unwrap().remove(0);
        info.active_mode = Some(
            *info
                .modes
                .iter()
                .find(|m| m.width == stream.width && m.height == stream.height)
                .unwrap(),
        );
        let path = std::env::join_paths(
            std::iter::once(directory.0.clone())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        let camera = RpicamCamera::new(info)
            .unwrap()
            .with_isp(listing::isp(listing).unwrap())
            .with_path(path);
        Self {
            camera,
            _directory: directory,
        }
    }
    async fn frame(&mut self) -> Result<GrayFrame, scope_node::CameraError> {
        timeout(Duration::from_secs(5), self.camera.next_frame())
            .await
            .expect("camera read must finish within 5 s")
    }
    async fn assert_exit(&mut self) {
        let error = self.frame().await.unwrap_err().to_string();
        assert!(error.contains("fake-camera-exit"), "{error}");
    }
}

#[test]
fn s3_both_stride_rules_equal_the_measured_values() {
    let measurements = fixture().measured_strides;
    assert_eq!(
        measurements
            .iter()
            .map(|m| (m.width, m.stride))
            .collect::<Vec<_>>(),
        [
            (640, 640),
            (1296, 1408),
            (1920, 1920),
            (2592, 2688),
            (1000, 1024),
            (1040, 1152)
        ]
    );
    for m in measurements {
        assert_eq!(
            Isp::Pisp.stride(m.width),
            m.stride,
            "Pi 5 width {}",
            m.width
        );
    }
    // fixtures/scope-node/README.md, The row stride: the Pi 3B+ measurements.
    for (width, expected) in [
        (640, 640),
        (1296, 1344),
        (1920, 1920),
        (1000, 1024),
        (1040, 1088),
    ] {
        assert_eq!(Isp::Vc4.stride(width), expected, "VC4 width {width}");
        assert_eq!(yuv::stride(width), expected, "legacy stride width {width}");
    }
}

#[test]
fn s4_first_camera_handler_takes_precedence_over_its_path() {
    let pi5 = saved_listing(true);
    let pi3 = saved_listing(false);
    let without_handler = pi5
        .lines()
        .filter(|l| !l.contains("PipelineHandler"))
        .collect::<Vec<_>>()
        .join("\n");
    let vc4_handler = pi5.replace("PipelineHandler : rpi/pisp", "PipelineHandler : rpi/vc4");
    let second_camera = pi5
        .lines()
        .skip_while(|l| !l.starts_with("0 : "))
        .collect::<Vec<_>>()
        .join("\n")
        .replacen("0 : ", "1 : ", 1);
    let two_cameras = format!("{pi3}\n{second_camera}");
    for (text, expected) in [
        (&pi5, Isp::Pisp),
        (&pi3, Isp::Vc4),
        (&without_handler, Isp::Pisp),
        (&vc4_handler, Isp::Vc4),
        (&two_cameras, Isp::Vc4),
    ] {
        assert_eq!(listing::isp(text).unwrap(), expected);
    }
    let unknown = pi5.replace("PipelineHandler : rpi/pisp", "PipelineHandler : rpi/other");
    assert_eq!(listing::isp(&unknown).unwrap_err().to_string(),
        "camera listing names the pipeline handler rpi/other, whose row stride the node does not know");
}

#[tokio::test]
async fn s6_pi5_adapter_preserves_saved_pixels_and_exposure_times_exactly() {
    let s = fixture().stream;
    let stream = Stream::saved(&s);
    let mut rig = Rig::new("s6-capture", &saved_listing(true), &stream);
    for i in 0..3 {
        let frame = rig.frame().await.unwrap();
        assert_eq!((frame.width, frame.height), (1296, 972));
        assert!(
            frame.pixels == stream.gray[i],
            "saved frame {i} pixels differ"
        );
        let start = s.sensor_timestamps_ns[i] as f64 / 1e9;
        assert_eq!(frame.start_s, start);
        assert_eq!(frame.end_s, start + s.exposure_times_us[i] as f64 / 1e6);
    }
    rig.assert_exit().await;
}

#[tokio::test]
async fn s7_all_listed_modes_frame_three_synthetic_images_on_both_isps() {
    for (name, pi5, modes) in [
        (
            "pisp",
            true,
            [
                (640, 480, 640),
                (1296, 972, 1408),
                (1920, 1080, 1920),
                (2592, 1944, 2688),
            ],
        ),
        (
            "vc4",
            false,
            [
                (640, 480, 640),
                (1296, 972, 1344),
                (1920, 1080, 1920),
                (2592, 1944, 2624),
            ],
        ),
    ] {
        for (width, height, stride) in modes {
            let stream = Stream::synthetic(width, height, stride, false);
            let mut rig = Rig::new(&format!("s7-{name}-{width}"), &saved_listing(pi5), &stream);
            for i in 0..3 {
                let frame = rig.frame().await.unwrap();
                assert_eq!((frame.width, frame.height), (width, height));
                assert!(
                    frame.pixels == stream.gray[i],
                    "{name} {width} frame {i} differs"
                );
            }
            rig.assert_exit().await;
        }
    }
}

#[tokio::test]
async fn s8_wrong_strides_and_later_nonzero_padding_are_refused_within_five_seconds() {
    for (name, pi5, stride, dark, message) in [
        (
            "s8-pisp-1472",
            true,
            1472,
            false,
            "frame 1 does not fit a 1408-byte row stride at width 1296: its row padding is not 0",
        ),
        (
            "s8-vc4-1408",
            false,
            1408,
            false,
            "frame 1 does not fit a 1344-byte row stride at width 1296: its row padding is not 0",
        ),
        (
            "s8-vc4-dark",
            false,
            1408,
            true,
            "frame 1 does not fit a 1344-byte row stride at width 1296: its row padding is not 0",
        ),
    ] {
        let stream = Stream::synthetic(1296, 972, stride, dark);
        let mut rig = Rig::new(name, &saved_listing(pi5), &stream);
        let error = rig.frame().await.unwrap_err().to_string();
        assert!(error.contains(message), "{name}: {error}");
    }
    let mut stream = Stream::synthetic(1296, 972, 1408, false);
    for row in stream.frames[1][..1408 * 972].as_chunks_mut::<1408>().0 {
        row[1296] = 1;
    }
    let mut rig = Rig::new("s8-second-frame", &saved_listing(true), &stream);
    assert!(rig.frame().await.unwrap().pixels == stream.gray[0]);
    let error = rig.frame().await.unwrap_err().to_string();
    assert!(
        error.contains(
            "frame 2 does not fit a 1408-byte row stride at width 1296: its row padding is not 0"
        ),
        "{error}"
    );
}

#[tokio::test]
async fn nonzero_luma_and_both_chroma_planes_return_invalid_data() {
    // 4x2 at stride 8: Y occupies 16 bytes, then one 4-byte row per U/V.
    // Probe the first and last padding byte of each plane independently.
    for position in [4, 7, 12, 15, 18, 19, 22, 23] {
        let mut bytes = [0; 24];
        bytes[position] = 1;
        let mut reader = YuvReader::new(bytes.as_slice(), 4, 2, 8).unwrap();
        let error = reader.next().await.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            error.to_string(),
            "frame 1 does not fit a 8-byte row stride at width 4: its row padding is not 0"
        );
    }
}
