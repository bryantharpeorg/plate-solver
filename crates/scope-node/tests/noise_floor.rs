//! US1-S6/S7: observer-facing config output and exact error messages.
use scope_node::config::Config;
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const HEAD: &str = "listen = '127.0.0.1:50051'\ntoken = 'test-token'\nadapter = 'simulated'\n";

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn run(config: &str) -> Output {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = Scratch(PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "scope-node-noise-floor-{}-{}.toml",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(path.0.parent().unwrap()).unwrap();
    std::fs::write(&path.0, config).unwrap();
    Command::new(env!("CARGO_BIN_EXE_scope-node"))
        .arg("--config")
        .arg(&path.0)
        .arg("--check-config")
        .output()
        .unwrap()
}

#[test]
fn floor_is_absent_by_default_and_printed_after_sigma_when_set() {
    let bare = run(HEAD);
    assert_eq!(bare.status.code(), Some(0));
    assert!(bare.stderr.is_empty());
    let stdout = String::from_utf8(bare.stdout).unwrap();
    // Literal captured from main's --check-config, before this story.
    assert!(stdout.contains("[detection]\nsigma = 8.0\nbinning = 2\nnormalize_rows = true\ndetect_hot_pixels = true\nreturn_binned = false\nuse_binned_for_star_candidates = true\n\n# no mount configured\n"), "{stdout}");
    assert!(!stdout.contains("noise_floor"));
    let config = Config::parse(HEAD).unwrap();
    assert_eq!(config.settings().detection.noise_floor, None);
    assert_eq!(config.detection.noise_floor, None);

    let source = format!("{HEAD}[detection]\nnoise_floor = 1.0\n");
    let raised = run(&source);
    assert_eq!(raised.status.code(), Some(0));
    assert!(raised.stderr.is_empty());
    let stdout = String::from_utf8(raised.stdout).unwrap();
    assert!(
        stdout.contains("[detection]\nsigma = 8.0\nnoise_floor = 1.0\nbinning = 2\n"),
        "{stdout}"
    );
    let config = Config::parse(&source).unwrap();
    assert_eq!(config.settings().detection.noise_floor, Some(1.0));
    assert_eq!(config.detection.noise_floor, Some(1.0));
}

#[test]
fn floor_range_is_inclusive_and_invalid_values_name_the_key() {
    for value in ["0.1", "5.5", "nan"] {
        let source = format!("{HEAD}[detection]\nnoise_floor = {value}\n");
        let output = run(&source);
        assert_eq!(output.status.code(), Some(1), "{value}");
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "scope-node: detection.noise_floor must be between 0.2 and 5\n",
            "{value}"
        );
    }
    let output = run(&format!("{HEAD}[detection]\nnoise_floor = 'high'\n"));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "scope-node: invalid noise_floor at line 5\n"
    );
    for value in ["0.2", "5"] {
        let source = format!("{HEAD}[detection]\nnoise_floor = {value}\n");
        let output = run(&source);
        assert_eq!(output.status.code(), Some(0), "{value}");
        assert!(output.stderr.is_empty(), "{value}");
    }
}
