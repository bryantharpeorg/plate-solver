use std::{
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn run(config: &str, check: bool) -> Output {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = Scratch(PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "scope-node-cli-{}-{}.toml",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(path.0.parent().unwrap()).unwrap();
    std::fs::write(&path.0, config).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_scope-node"));
    command.arg("--config").arg(&path.0);
    if check {
        command.arg("--check-config");
    }
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
    child.wait_with_output().unwrap()
}

const CONFIG: &str = "listen = '127.0.0.1:50051'\ntoken = 'test-token'\nadapter = 'simulated'\nexposure_ms = 20\ngain = 'auto'\n";

#[test]
fn check_config_prints_every_setting_and_masks_the_token() {
    let output = run(CONFIG, true);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for expected in [
        "listen",
        "127.0.0.1:50051",
        "token",
        "****",
        "adapter",
        "simulated",
        "exposure_ms",
        "20",
        "gain",
        "auto",
        "detection",
        "sigma",
        "binning",
        "normalize_rows",
        "detect_hot_pixels",
    ] {
        assert!(stdout.contains(expected), "{expected} absent from {stdout}");
    }
    assert!(!stdout.contains("test-token"));
    assert!(!stderr.contains("test-token"));
}

#[test]
fn start_refuses_unknown_keys_and_bad_values_naming_the_key() {
    for (config, key) in [
        (CONFIG.replace("exposure_ms", "exposre_ms"), "exposre_ms"),
        (CONFIG.replace("gain = 'auto'", "gain = 'loud'"), "gain"),
    ] {
        let output = run(&config, false);
        assert!(!output.status.success());
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(message.contains(key), "{message}");
        assert!(
            !message.contains("test-token"),
            "config errors must not echo the secret source line"
        );
    }
}
