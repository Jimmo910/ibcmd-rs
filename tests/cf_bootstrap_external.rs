//! `cf bootstrap --base-free` of an external data processor or report from
//! its XML tree alone (issue #438): a fixture package exported, built back
//! from the tree with no base, and exported again gives the same tree.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "ibcmd-cf-bootstrap-external-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn ibcmd(args: &[&Path]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ibcmd-rs"));
    for arg in args {
        command.arg(arg);
    }
    command.output().unwrap()
}

fn export(file: &Path, tree: &Path) {
    let out = ibcmd(&[
        Path::new("cf"),
        Path::new("export"),
        file,
        tree,
        Path::new("--overwrite"),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn files(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.push((relative, fs::read(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

fn round_trip(fixture: &str, suffix: &str, tag: &str) {
    let scratch = Scratch::new(tag);
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture);
    let tree = scratch.0.join("tree");
    export(&fixture, &tree);
    let output = scratch.0.join(format!("built.{suffix}"));
    let built = ibcmd(&[
        Path::new("cf"),
        Path::new("bootstrap"),
        Path::new("--base-free"),
        Path::new("--platform"),
        Path::new("8.3.27"),
        &tree,
        &output,
    ]);
    assert!(
        built.status.success(),
        "{}{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    let back = scratch.0.join("back");
    export(&output, &back);
    assert_eq!(files(&tree), files(&back));
}

#[test]
fn a_processor_builds_from_its_tree_alone() {
    round_trip(
        "tests/fixtures/external/test_processor/input.epf",
        "epf",
        "processor",
    );
}

#[test]
fn a_report_builds_from_its_tree_alone() {
    round_trip(
        "tests/fixtures/external/test_report/input.erf",
        "erf",
        "report",
    );
}
