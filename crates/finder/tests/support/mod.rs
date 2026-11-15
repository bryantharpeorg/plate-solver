//! Helpers shared by finder's integration-test binaries. Nothing under this directory
//! is a test: a test function is compiled into exactly one binary.
pub mod goto;
pub mod live;
pub mod mcp;
pub mod stop;
pub mod tracker;

/// A folder for one test's files, under the target directory cargo names, removed
/// when dropped. The name, process id and a counter keep it unique under nextest and
/// under `cargo test`.
pub struct Scratch(pub std::path::PathBuf);
impl Scratch {
    pub fn new(name: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
