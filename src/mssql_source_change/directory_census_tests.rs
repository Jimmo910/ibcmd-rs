//! Strict directory membership controls; no compiler or database fixture.
use super::*;

// No Drop cleanup: all paths are newly created public test artifacts retained
// for inspection. Removal from the source census uses an owned parking sibling.
struct Fixture {
    root: PathBuf,
    source: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("ibcmd-directory-census-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        fs::create_dir(&source).unwrap();
        fs::create_dir(root.join("parking")).unwrap();
        fs::write(source.join("keep.bin"), b"unchanged public bytes").unwrap();
        fs::create_dir(source.join("empty-before")).unwrap();
        Self { root, source }
    }
    fn capture(&self) -> HeldSourceRoot {
        HeldSourceRoot::open_with_traversal_limits(
            &self.source,
            SourceInventoryLimits {
                max_files: 8,
                max_file_bytes: 1024,
                max_total_bytes: 4096,
            },
            SourceTraversalLimits {
                max_entries: 32,
                max_depth: 8,
            },
        )
        .unwrap()
    }
    fn assert_refused(&self, held: &HeldSourceRoot) {
        assert_eq!(
            fs::read(self.source.join("keep.bin")).unwrap(),
            b"unchanged public bytes"
        );
        assert!(
            held.require_unchanged().is_err(),
            "directory membership changed below traversal limits but revalidation accepted it"
        );
    }
}

#[test]
fn unchanged_directory_census_is_accepted() {
    let fixture = Fixture::new();
    let held = fixture.capture();
    held.require_unchanged().unwrap();
    held.require_unchanged().unwrap();
}

#[test]
fn empty_directory_addition_invalidates_held_census() {
    let fixture = Fixture::new();
    let held = fixture.capture();
    fs::create_dir(fixture.source.join("empty-after")).unwrap();
    fixture.assert_refused(&held);
}

#[test]
fn empty_directory_membership_removal_invalidates_held_census() {
    let fixture = Fixture::new();
    let held = fixture.capture();
    fs::rename(
        fixture.source.join("empty-before"),
        fixture.root.join("parking/empty-before"),
    )
    .unwrap();
    fixture.assert_refused(&held);
}

#[test]
fn empty_directory_rename_invalidates_held_census() {
    let fixture = Fixture::new();
    let held = fixture.capture();
    fs::rename(
        fixture.source.join("empty-before"),
        fixture.source.join("renamed-after"),
    )
    .unwrap();
    fixture.assert_refused(&held);
}

#[test]
fn nested_empty_directory_rename_invalidates_held_census() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.source.join("empty-before/nested-before")).unwrap();
    let held = fixture.capture();
    fs::rename(
        fixture.source.join("empty-before/nested-before"),
        fixture.source.join("empty-before/nested-after"),
    )
    .unwrap();
    fixture.assert_refused(&held);
}

#[test]
fn case_only_empty_directory_rename_invalidates_held_census() {
    let fixture = Fixture::new();
    let held = fixture.capture();
    fs::rename(
        fixture.source.join("empty-before"),
        fixture.source.join("EMPTY-before"),
    )
    .unwrap();
    assert!(
        fs::read_dir(&fixture.source)
            .unwrap()
            .any(|entry| entry.unwrap().file_name() == "EMPTY-before")
    );
    fixture.assert_refused(&held);
}

#[test]
fn directory_replaced_by_file_is_not_unchanged() {
    let fixture = Fixture::new();
    let held = fixture.capture();
    fs::rename(
        fixture.source.join("empty-before"),
        fixture.root.join("parking/empty-before"),
    )
    .unwrap();
    fs::write(
        fixture.source.join("empty-before"),
        b"different member type",
    )
    .unwrap();
    fixture.assert_refused(&held);
}
