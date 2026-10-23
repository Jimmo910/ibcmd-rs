//! Only fresh, uniquely named test artifacts are owned and removed here.
use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};

pub struct OwnedScratch(PathBuf);
impl OwnedScratch {
    pub fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Deref for OwnedScratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}
impl Drop for OwnedScratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn owned_fixture_cleanup_covers_success_and_unwind() {
    let path = {
        let scratch = OwnedScratch::new("issue382-cleanup-success");
        fs::write(scratch.join("fixture"), b"owned").unwrap();
        scratch.to_path_buf()
    };
    assert!(!path.exists());
    let mut path = None;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let scratch = OwnedScratch::new("issue382-cleanup-unwind");
            path = Some(scratch.to_path_buf());
            let _file = fs::File::create(scratch.join("fixture")).unwrap();
            panic!("synthetic fixture failure");
        }))
        .is_err()
    );
    assert!(!path.unwrap().exists());
}
