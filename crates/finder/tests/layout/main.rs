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
