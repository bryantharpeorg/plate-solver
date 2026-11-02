use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

fn check(mount: &str) -> std::process::Output {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/mount-config-{}-{}.toml",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        format!("token = 'test-token'\nadapter = 'simulated'\n{mount}"),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_scope-node"))
        .arg("--config")
        .arg(&path)
        .arg("--check-config")
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    output
}

#[test]
fn optional_mount_device_is_reported_without_opening_it() {
    for (table, expected) in [
        ("[mount]\ndevice = '/dev/ttyUSB0'", "/dev/ttyUSB0"),
        ("", "no mount configured"),
    ] {
        let output = check(table);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains(expected));
    }
}

#[test]
fn mount_rejects_unknown_keys_and_missing_or_empty_device() {
    for (table, key) in [
        ("[mount]\ndevice = '/dev/ttyUSB0'\ndevcie = 'bad'", "devcie"),
        ("[mount]", "device"),
        ("[mount]\ndevice = ''", "device"),
    ] {
        let output = check(table);
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(key), "{error}");
        assert!(!error.contains("test-token"));
    }
}
