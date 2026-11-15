//! Where finder's integration tests live, checked.
//!
//! The rules (specs/027-finder-tests-by-system): a test function is compiled into one
//! test binary, shared helpers live under `tests/support/` and hold no tests, and the
//! tests are grouped into one binary per system. These tests read the source tree; the
//! oracle is the rule, not what the tree happens to hold.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// A line that makes the next function a test.
fn declares_a_test(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("#[test]") || line.starts_with("#[tokio::test")
    })
}

/// The files a source file pulls in with a path attribute at the start of a line.
fn included(file: &Path) -> Vec<PathBuf> {
    let text = fs::read_to_string(file).unwrap();
    text.lines()
        .filter_map(|line| line.trim_start().strip_prefix("#[path = \""))
        .filter_map(|rest| rest.split('"').next())
        .map(|relative| file.parent().unwrap().join(relative))
        .collect()
}

#[test]
fn no_file_with_tests_is_included_by_another() {
    let mut files = Vec::new();
    rust_files(&tests_dir(), &mut files);
    assert!(
        files.len() > 40,
        "the scan found only {} files",
        files.len()
    );
    let mut includes = 0;
    for file in &files {
        for target in included(file) {
            includes += 1;
            let text = fs::read_to_string(&target).unwrap_or_else(|e| {
                panic!("{} includes {}: {e}", file.display(), target.display())
            });
            assert!(
                !declares_a_test(&text),
                "{} includes {}, which holds tests: they would run once in each binary",
                file.display(),
                target.display()
            );
        }
    }
    assert!(includes > 0, "the scan found no path attribute at all");
}

#[test]
fn support_holds_helpers_and_no_tests() {
    let support = tests_dir().join("support");
    for name in ["goto.rs", "tracker.rs"] {
        assert!(
            support.join(name).is_file(),
            "tests/support/{name} is missing"
        );
    }
    let mut files = Vec::new();
    rust_files(&support, &mut files);
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        assert!(!declares_a_test(&text), "{} holds a test", file.display());
    }
}

/// The lines of a source file that are neither blank nor comments.
fn code_lines(file: &Path) -> Vec<String> {
    fs::read_to_string(file)
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn tests_are_grouped_into_one_binary_per_system() {
    let mut entries: Vec<String> = fs::read_dir(tests_dir())
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(
                entry.path().is_dir(),
                "tests/{} is a file: a test belongs in a module of cli, console, mcp or motion",
                entry.file_name().to_string_lossy()
            );
            entry.file_name().to_string_lossy().into_owned()
        })
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        ["cli", "console", "layout", "mcp", "motion", "support"]
    );

    let mut helpers: Vec<String> = fs::read_dir(tests_dir().join("support"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    helpers.sort();
    assert_eq!(
        helpers,
        [
            "goto.rs",
            "live.rs",
            "mcp.rs",
            "mod.rs",
            "stop.rs",
            "tracker.rs"
        ]
    );

    for binary in ["cli", "console", "mcp", "motion"] {
        let main = tests_dir().join(binary).join("main.rs");
        let lines = code_lines(&main);
        assert_eq!(
            lines[..3],
            [
                "#[allow(dead_code, unused_imports)]",
                "#[path = \"../support/mod.rs\"]",
                "mod support;"
            ],
            "{binary}/main.rs starts with the shared support tree"
        );
        for line in &lines[3..] {
            let module = line
                .strip_prefix("mod ")
                .and_then(|rest| rest.strip_suffix(';'));
            assert!(
                module.is_some_and(|name| name.chars().all(|c| c.is_ascii_lowercase() || c == '_')),
                "{binary}/main.rs holds only module declarations, found: {line}"
            );
        }
        assert!(lines.len() > 3, "{binary}/main.rs declares no module");
    }
}
