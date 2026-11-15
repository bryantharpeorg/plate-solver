//! The `finder` binary's command line.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

const NOTICES: &str = include_str!("../../THIRD_PARTY_NOTICES.md");

/// `--licenses` prints the third-party notices and exits before loading a database.
#[test]
fn licenses_prints_the_notices_and_exits() {
    let out = Command::new(env!("CARGO_BIN_EXE_finder"))
        .args(["--licenses", "--db", "no-such-database.npz"])
        .output()
        .expect("the finder binary runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8(out.stdout).unwrap(), NOTICES);
}

/// The notices give each package in the page's build the version and license that
/// `web/package-lock.json` pins, so an upgrade that leaves the notices behind fails here.
#[test]
fn the_notices_name_the_page_packages_as_locked() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("web/package-lock.json");
    let lock: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for (package, row) in [
        ("react", "(`react`)"),
        ("react-dom", "(`react-dom`)"),
        ("scheduler", "`scheduler`"),
        ("tailwindcss", "[Tailwind CSS]"),
        ("vite", "[Vite]"),
        (
            "@fontsource-variable/archivo",
            "`@fontsource-variable/archivo`",
        ),
    ] {
        let locked = &lock["packages"][format!("node_modules/{package}")];
        let version = locked["version"].as_str().unwrap();
        let license = locked["license"].as_str().unwrap();
        let line = NOTICES
            .lines()
            .find(|l| l.starts_with('|') && l.contains(row))
            .unwrap_or_else(|| panic!("no row for {package}"));
        assert!(
            line.ends_with(&format!("| {version} | {license} |")),
            "{package} is locked at {version} ({license}): {line}"
        );
    }
}
