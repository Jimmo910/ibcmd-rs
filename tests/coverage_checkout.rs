//! The corpus is a required pinned input, never an optional test skip.
use std::{path::PathBuf, process::Command};

#[test]
fn coverage_checkout_and_converter_expectations_are_valid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let python = if cfg!(windows) { "python" } else { "python3" };
    let result = Command::new(python)
        .arg(root.join("tools/coverage/fixture_paths.py"))
        .current_dir(root)
        .output()
        .expect("Python is required to validate the pinned corpus");
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
