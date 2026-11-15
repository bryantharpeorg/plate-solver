//! OpenNGC's data provenance and license remain tied to the bundled extract.

use std::path::Path;

#[test]
fn openngc_notice_names_the_license_and_the_pinned_commit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = std::fs::read_to_string(root.join("data/README.md")).unwrap();
    let extract_notes = readme.split("## `openngc.csv`").nth(1).unwrap();
    let commit = extract_notes
        .split('`')
        .find(|word| word.len() == 40 && word.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .expect("OpenNGC's pinned commit in the extraction recipe");
    let notices = std::fs::read_to_string(root.join("THIRD_PARTY_NOTICES.md")).unwrap();
    let row = notices
        .lines()
        .find(|line| line.starts_with("| `openngc.csv` |"))
        .expect("a notice for the full OpenNGC extract");
    assert!(row.contains("OpenNGC"));
    assert!(row.contains("Mattia Verga"));
    assert!(row.contains("CC BY-SA 4.0"));
    assert!(row.contains(commit), "the notice must pin {commit}: {row}");
}
