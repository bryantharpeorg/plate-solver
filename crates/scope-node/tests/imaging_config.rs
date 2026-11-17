//! US5-S3 and US5-S4: the `[imaging]` table parses with its defaults and its
//! neutral controls, refuses what it must without printing the token, and
//! the built `scope-node --check-config` prints it, or says no imaging
//! camera is configured.

use scope_node::config::{Config, ImagingAdapter};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

/// A config file under `CARGO_TARGET_TMPDIR`, removed on drop (the shared
/// setup's scratch rule).
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Parse a config of the token and the given imaging table, answering the
/// error's text.
fn parse(table: &str) -> Result<Config, String> {
    Config::parse(&format!(
        "listen = '127.0.0.1:50051'\ntoken = 'test-token'\n{table}"
    ))
    .map_err(|error| error.to_string())
}

/// Run the built `scope-node --check-config` as `crates/scope-node/tests/cli.rs`
/// runs it, on a config file kept as the shared setup's scratch rule says.
fn run(config: &str) -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = Scratch(PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "imaging-config-{}-{}.toml",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(path.0.parent().unwrap()).unwrap();
    std::fs::write(&path.0, config).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_scope-node"))
        .arg("--config")
        .arg(&path.0)
        .arg("--check-config")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// US5-S3: a config with no `[imaging]` table has `imaging` `None`.
#[test]
fn a_config_without_the_imaging_table_has_none() {
    let config = parse("").unwrap();
    assert!(config.imaging.is_none());
}

/// US5-S3: the `indi` table takes its address, its device and its neutral
/// table; alone it has INDI's own address and port, and no device.
#[test]
fn the_indi_table_takes_its_address_device_and_neutral() {
    let config = parse("[imaging]\nadapter = 'indi'\n").unwrap();
    let imaging = config.imaging.as_ref().unwrap();
    assert_eq!(imaging.adapter, ImagingAdapter::Indi);
    assert_eq!(imaging.address().to_string(), "127.0.0.1:7624");
    assert_eq!(imaging.device, None);
    assert!(imaging.neutral.is_empty());
    assert!(imaging.neutral_controls().is_empty());

    let config = parse(
        "[imaging]\nadapter = 'indi'\naddress = '127.0.0.1:7700'\ndevice = 'CCD Simulator'\n",
    )
    .unwrap();
    let imaging = config.imaging.as_ref().unwrap();
    assert_eq!(imaging.address().to_string(), "127.0.0.1:7700");
    assert_eq!(imaging.device.as_deref(), Some("CCD Simulator"));
}

/// US5-S3: the `simulated` adapter parses and refuses the `indi` keys.
#[test]
fn the_simulated_adapter_refuses_the_indi_keys() {
    let config = parse("[imaging]\nadapter = 'simulated'\n").unwrap();
    assert_eq!(
        config.imaging.as_ref().unwrap().adapter,
        ImagingAdapter::Simulated
    );
    for (table, key) in [
        (
            "[imaging]\nadapter = 'simulated'\naddress = '127.0.0.1:7700'\n",
            "imaging.address",
        ),
        (
            "[imaging]\nadapter = 'simulated'\ndevice = 'CCD Simulator'\n",
            "imaging.device",
        ),
    ] {
        let error = parse(table).err().unwrap();
        assert!(error.contains(key), "{error}");
        assert!(!error.contains("test-token"), "{error}");
    }
}

/// US5-S3: an empty table is refused naming the missing adapter, and an
/// unknown adapter, an empty device and an unknown key are refused naming
/// the key, never the token.
#[test]
fn the_table_refuses_what_it_must_naming_the_key() {
    for (table, key) in [
        ("[imaging]\n", "missing imaging.adapter"),
        ("[imaging]\nadapter = 'usb'\n", "adapter"),
        ("[imaging]\nadapter = 'indi'\ndevice = ''\n", "device"),
        ("[imaging]\nadapter = 'indi'\nexposure = 5\n", "exposure"),
    ] {
        let error = parse(table).err().unwrap();
        assert!(error.contains(key), "{error}");
        assert!(!error.contains("test-token"), "{error}");
    }
}

/// US5-S3: the neutral table holds its values, answers them as
/// `(property, element, value)` in the table's sorted order and splits each
/// key at its first dot only.
#[test]
fn the_neutral_table_answers_its_controls_split_at_the_first_dot() {
    let config = parse(
        "[imaging]\nadapter = 'indi'\nneutral = { \"CCD_CONTROLS.WB_R\" = 50, \"CCD_CONTROLS.WB_B\" = 50 }\n",
    )
    .unwrap();
    let imaging = config.imaging.as_ref().unwrap();
    let table: BTreeMap<String, f64> = [
        ("CCD_CONTROLS.WB_R".to_string(), 50.0),
        ("CCD_CONTROLS.WB_B".to_string(), 50.0),
    ]
    .into_iter()
    .collect();
    assert_eq!(imaging.neutral, table);
    assert_eq!(
        imaging.neutral_controls(),
        vec![
            ("CCD_CONTROLS".to_string(), "WB_B".to_string(), 50.0),
            ("CCD_CONTROLS".to_string(), "WB_R".to_string(), 50.0),
        ]
    );

    // A key with more than one dot splits at its first only.
    let config =
        parse("[imaging]\nadapter = 'indi'\nneutral = { \"CCD_CONTROLS.WB.R\" = 1 }\n").unwrap();
    assert_eq!(
        config.imaging.as_ref().unwrap().neutral_controls(),
        vec![("CCD_CONTROLS".to_string(), "WB.R".to_string(), 1.0)]
    );
}

/// US5-S3: the neutral table's refusals, each with its exact message.
#[test]
fn the_neutral_table_is_checked_with_exact_messages() {
    for (table, expected) in [
        (
            "[imaging]\nadapter = 'simulated'\nneutral = { \"CCD_CONTROLS.WB_R\" = 50 }\n",
            "imaging.neutral applies only to the indi adapter",
        ),
        (
            "[imaging]\nadapter = 'indi'\nneutral = { \"WB_R\" = 50 }\n",
            "imaging.neutral key WB_R must be PROPERTY.ELEMENT",
        ),
        (
            "[imaging]\nadapter = 'indi'\nneutral = { \"CCD_CONTROLS.WB_R\" = nan }\n",
            "imaging.neutral CCD_CONTROLS.WB_R must be a finite number",
        ),
    ] {
        assert_eq!(parse(table).err().unwrap(), expected);
    }
}

/// US5-S4: `--check-config` prints the `[imaging]` table with its neutral
/// line, and on a config with no such table says no imaging camera is
/// configured.
#[test]
fn check_config_prints_the_imaging_table_or_says_none_is_configured() {
    let stdout = run(
        "listen = '127.0.0.1:50051'\ntoken = 'test-token'\nadapter = 'simulated'\n\n\
         [imaging]\nadapter = 'indi'\ndevice = 'CCD Simulator'\n\
         neutral = { \"CCD_CONTROLS.WB_R\" = 50, \"CCD_CONTROLS.WB_B\" = 50 }\n",
    );
    for expected in [
        "[imaging]",
        "adapter = \"indi\"",
        "address = \"127.0.0.1:7624\"",
        "device = \"CCD Simulator\"",
        "neutral = { \"CCD_CONTROLS.WB_B\" = 50, \"CCD_CONTROLS.WB_R\" = 50 }",
    ] {
        assert!(stdout.contains(expected), "{expected} absent from {stdout}");
    }
    assert!(!stdout.contains("test-token"));

    let stdout = run("listen = '127.0.0.1:50051'\ntoken = 'test-token'\n");
    assert!(
        stdout.contains("# no imaging camera configured"),
        "the absent table is named: {stdout}"
    );
}
