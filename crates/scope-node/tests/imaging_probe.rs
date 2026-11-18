//! US8's scenarios, one test each. Oracles: the stand-in camera's fixed
//! description (plan.md § The stand-in camera), every expected line spelled out
//! here rather than read from the probe, and the worked example of US8-S2: a
//! frame answered 350 ms after a 100 ms exposure was asked is 250 ms in hand.

use scope_node::{
    clock::NodeClock,
    imaging::{
        probe, synthetic::SyntheticImagingCamera, Exposure, ImagingCamera, ImagingError,
        ImagingFrame, ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus, Range, Roi,
    },
};
use std::{
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

// The test clock of spec.md § User Scenarios: 1000.0 plus the tokio time
// elapsed since it was built, so paused time advances it exactly.
struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        1000.0 + self.0.elapsed().as_secs_f64()
    }
}

/// A config file under `CARGO_TARGET_TMPDIR`, removed on drop, as the shared
/// setup's scratch rule asks.
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Run the built `scope-node` on a config file inside this test's scratch
/// folder, as `run` in `crates/scope-node/tests/cli.rs` runs it, and answer
/// its output and how long it took.
fn run(config: &str, args: &[&str]) -> (Output, Duration) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = Scratch(PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "scope-node-probe-{}-{}.toml",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(path.0.parent().unwrap()).unwrap();
    std::fs::write(&path.0, config).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_scope-node"));
    command.arg("--config").arg(&path.0).args(args);
    let started = Instant::now();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() >= Duration::from_secs(10) {
            let _ = child.kill();
            child.wait().unwrap();
            panic!(
                "scope-node did not exit within 10 s: {:?}",
                command.get_args().collect::<Vec<_>>()
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let output = child.wait_with_output().unwrap();
    (output, started.elapsed())
}

/// The stand-in camera's eight description lines, as US8-S1 spells them out.
const STAND_IN: &str = "imaging camera: synthetic imaging camera\n\
     serial: none\n\
     sensor: 64 x 48 px, 4.78 um pixels, 16 bits, RGGB\n\
     exposure: 1 to 60000 ms\n\
     gain: 0 to 300\n\
     offset: 0 to 100\n\
     binnings: 1, 2\n\
     cooler: yes\n";

/// A camera of the test's own (US8-S2): mono, serial `A1`, no gain, offset or
/// cooler, that answers each exposure `answer_ms` after it was asked with a
/// 3 × 2, 8-bit frame. `answers` exposures answer; a later one fails with
/// `the camera fell over`.
struct SlowMono {
    clock: Arc<dyn NodeClock>,
    answer_ms: f64,
    answers: usize,
    taken: AtomicUsize,
}
impl SlowMono {
    fn new(clock: Arc<dyn NodeClock>, answer_ms: f64, answers: usize) -> Self {
        Self {
            clock,
            answer_ms,
            answers,
            taken: AtomicUsize::new(0),
        }
    }
}
impl ImagingCamera for SlowMono {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        Box::pin(async {
            Ok(ImagingInfo {
                model: "mono probe camera".into(),
                serial: Some("A1".into()),
                width: 3,
                height: 2,
                pixel_um: 5.2,
                bits: 8,
                bayer: None,
                exposure_ms: Range {
                    min: 1.0,
                    max: 60_000.0,
                },
                gain: None,
                offset: None,
                binnings: vec![1],
                cooler: false,
            })
        })
    }
    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        Box::pin(async {
            Ok(ImagingStatus {
                temp_c: None,
                cooler_power: None,
                set_temp_c: None,
                exposing: false,
            })
        })
    }
    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        Box::pin(async move { Ok(settings.clone()) })
    }
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async move {
            if self.taken.fetch_add(1, Ordering::SeqCst) >= self.answers {
                return Err(ImagingError::Failed("the camera fell over".into()));
            }
            let start_s = self.clock.now();
            let answer_ms = self.answer_ms;
            tokio::time::sleep(Duration::from_millis(answer_ms as u64)).await;
            Ok(ImagingFrame {
                samples: vec![1, 2, 3, 4, 5, 6],
                width: 3,
                height: 2,
                bits: 8,
                bayer: None,
                start_s,
                end_s: start_s + exposure.exposure_ms / 1000.0,
                exposure_ms: exposure.exposure_ms,
                gain: None,
                offset: None,
                binning: 1,
                temp_c: None,
                roi: Roi {
                    x: 0,
                    y: 0,
                    width: 3,
                    height: 2,
                },
            })
        })
    }
    fn abort(&self) -> ImagingFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }
}

/// US8-S1: the probe writes the stand-in camera's description and, for each of
/// three 500 ms exposures, the frame's size and how long the frame was in
/// hand, in exactly 1.5 s of virtual time.
#[tokio::test(start_paused = true)]
async fn the_probe_writes_the_stand_in_description_and_three_timed_exposures() {
    let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
    let camera = SyntheticImagingCamera::new(clock.clone());
    let mut out = Vec::new();
    let before = clock.now();
    probe::run(&camera, clock.as_ref(), 500.0, &mut out)
        .await
        .unwrap();
    // Three exposures of 500 ms, and nothing else that waits.
    assert_eq!(clock.now(), before + 1.5);
    let text = String::from_utf8(out).unwrap();
    assert_eq!(
        text,
        format!(
            "{STAND_IN}\
             exposure 1: 64 x 48, 6144 bytes, in hand 0 ms after the end of the exposure\n\
             exposure 2: 64 x 48, 6144 bytes, in hand 0 ms after the end of the exposure\n\
             exposure 3: 64 x 48, 6144 bytes, in hand 0 ms after the end of the exposure\n"
        )
    );
}

/// US8-S2: a camera of the test's own prints its own facts — a mono camera
/// with a serial, no gain, offset or cooler — and its download time is counted
/// from the end of the exposure (the worked example: 350 − 100 = 250). A
/// camera whose second exposure fails ends the run with that error, the
/// description and the first exposure's line written and nothing after.
#[tokio::test(start_paused = true)]
async fn a_camera_of_the_tests_own_prints_its_facts_and_a_failure_ends_the_run() {
    let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
    let camera = SlowMono::new(clock.clone(), 350.0, 3);
    let mut out = Vec::new();
    probe::run(&camera, clock.as_ref(), 100.0, &mut out)
        .await
        .unwrap();
    let text = String::from_utf8(out).unwrap();
    assert_eq!(
        text,
        "imaging camera: mono probe camera\n\
         serial: A1\n\
         sensor: 3 x 2 px, 5.2 um pixels, 8 bits, mono\n\
         exposure: 1 to 60000 ms\n\
         gain: none\n\
         offset: none\n\
         binnings: 1\n\
         cooler: no\n\
         exposure 1: 3 x 2, 6 bytes, in hand 250 ms after the end of the exposure\n\
         exposure 2: 3 x 2, 6 bytes, in hand 250 ms after the end of the exposure\n\
         exposure 3: 3 x 2, 6 bytes, in hand 250 ms after the end of the exposure\n"
    );

    let camera = SlowMono::new(clock.clone(), 350.0, 1);
    let mut out = Vec::new();
    assert_eq!(
        probe::run(&camera, clock.as_ref(), 100.0, &mut out).await,
        Err(ImagingError::Failed("the camera fell over".into()))
    );
    let text = String::from_utf8(out).unwrap();
    assert_eq!(
        text,
        "imaging camera: mono probe camera\n\
         serial: A1\n\
         sensor: 3 x 2 px, 5.2 um pixels, 8 bits, mono\n\
         exposure: 1 to 60000 ms\n\
         gain: none\n\
         offset: none\n\
         binnings: 1\n\
         cooler: no\n\
         exposure 1: 3 x 2, 6 bytes, in hand 250 ms after the end of the exposure\n"
    );
}

/// US8-S3: the built `scope-node --probe-camera --probe-exposure-ms 50` prints
/// the stand-in camera's description and three exposure lines and exits 0
/// within 2.5 s of real time, which three exposures of the default 1000 ms
/// cannot be. Without an `[imaging]` table it exits non-zero naming the table,
/// and `--probe-camera` with `--check-config` is refused by the argument
/// parser.
#[test]
fn the_built_node_probes_the_camera_and_refuses_without_a_table() {
    let imaging = "listen = '127.0.0.1:50051'\ntoken = 'test-token'\n\
                   adapter = 'simulated'\n[imaging]\nadapter = 'simulated'\n";
    let no_imaging = "listen = '127.0.0.1:50051'\ntoken = 'test-token'\n\
                      adapter = 'simulated'\n";

    let (output, took) = run(imaging, &["--probe-camera", "--probe-exposure-ms", "50"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        took < Duration::from_millis(2500),
        "the probe took {took:?}"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(STAND_IN),
        "the description is absent from {stdout}"
    );
    assert!(!stdout.contains("test-token"));
    let exposures: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("exposure "))
        .collect();
    assert_eq!(exposures.len(), 3, "{stdout}");
    for (n, line) in exposures.iter().enumerate() {
        let prefix = format!("exposure {}: 64 x 48, 6144 bytes, in hand ", n + 1);
        let ms = line
            .strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix(" ms after the end of the exposure"))
            .unwrap_or_else(|| panic!("{line} is not an exposure line"));
        assert!(
            !ms.is_empty() && ms.bytes().all(|byte| byte.is_ascii_digit()),
            "{line} holds no millisecond count"
        );
    }

    let (output, _) = run(no_imaging, &["--probe-camera"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("scope-node: no [imaging] table in the config"),
        "{stderr}"
    );

    let (output, _) = run(imaging, &["--probe-camera", "--check-config"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("cannot be used with"), "{stderr}");
}
