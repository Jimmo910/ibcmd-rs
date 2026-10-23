//! `infobase config export|import --db-path` (#373–#375) on file infobases
//! written by 1C:Enterprise 8.5.1.1150 (`tests/fixtures/file-infobase`). No
//! platform runs here: the export is compared with the platform's export of
//! the same file, and an import is read back through the export.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/file-infobase")
}

/// A fresh scratch directory for one test, removed on drop.
struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if std::env::var_os("IBCMD_TEST_KEEP_FIXTURES").as_deref()
            == Some(std::ffi::OsStr::new("1"))
        {
            return;
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let base = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-fixtures"));
    fs::create_dir_all(&base).unwrap();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = base.join(format!(
        "ibcmd-file-infobase-{}-{tag}-{nanos}",
        std::process::id()
    ));
    fs::create_dir(&dir).unwrap();
    Scratch(dir)
}

/// Unpacks `<name>.1CD.gz` into `<dir>/<name>/1Cv8.1CD`; returns the infobase
/// directory.
fn infobase(dir: &Path, name: &str) -> PathBuf {
    let mut packed = Vec::new();
    flate2::read::GzDecoder::new(
        fs::File::open(fixtures().join(format!("{name}.1CD.gz"))).unwrap(),
    )
    .read_to_end(&mut packed)
    .unwrap();
    let db = dir.join(name);
    fs::create_dir_all(&db).unwrap();
    fs::write(db.join("1Cv8.1CD"), packed).unwrap();
    db
}

fn run(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ibcmd-rs"))
        .args(args)
        .output()
        .expect("run ibcmd-rs")
}

fn ok(output: &Output) {
    assert!(
        output.status.success(),
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn db_path(db: &Path) -> std::ffi::OsString {
    let mut arg = std::ffi::OsString::from("--db-path=");
    arg.push(db);
    arg
}

fn export(db: &Path, out: &Path) -> Output {
    run(&[
        "infobase".as_ref(),
        "config".as_ref(),
        "export".as_ref(),
        &db_path(db),
        out.as_os_str(),
    ])
}

fn import(db: &Path, tree: &Path) -> Output {
    run(&[
        "infobase".as_ref(),
        "config".as_ref(),
        "import".as_ref(),
        &db_path(db),
        tree.as_os_str(),
    ])
}

fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (relative, fs::read(entry.path()).unwrap())
        })
        .collect()
}

/// The trees hold the same files, byte for byte, but for `ignored`.
fn assert_same_tree(expected: &Path, actual: &Path, ignored: &[&str]) {
    let keep = |map: BTreeMap<String, Vec<u8>>| {
        map.into_iter()
            .filter(|(path, _)| !ignored.contains(&path.as_str()))
            .collect::<BTreeMap<_, _>>()
    };
    let (expected, actual) = (keep(files(expected)), keep(files(actual)));
    let missing: Vec<_> = expected
        .keys()
        .filter(|k| !actual.contains_key(*k))
        .collect();
    let extra: Vec<_> = actual
        .keys()
        .filter(|k| !expected.contains_key(*k))
        .collect();
    let differ: Vec<_> = expected
        .iter()
        .filter(|(k, v)| actual.get(*k).is_some_and(|a| a != *v))
        .map(|(k, _)| k)
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty() && differ.is_empty(),
        "missing {missing:?}, extra {extra:?}, differ {differ:?}"
    );
}

#[test]
fn the_export_reproduces_the_platforms_export_of_the_file() {
    let dir = scratch("export");
    let db = infobase(&dir, "tiny");
    let out = dir.join("xml");
    ok(&export(&db, &out));
    assert_same_tree(&fixtures().join("tiny-xml"), &out, &[]);
    // The data file is read, never written.
    assert!(!ibcmd_filedb::backup_file(&db).exists());
}

#[test]
fn an_import_into_an_empty_infobase_stages_the_whole_tree() {
    let dir = scratch("import-empty");
    let db = infobase(&dir, "empty");
    let output = import(&db, &fixtures().join("tiny-xml"));
    ok(&output);
    assert!(
        ibcmd_filedb::backup_file(&db).is_file(),
        "the original is kept"
    );
    let mut staged = ibcmd_filedb::FileDb::open(&db).unwrap();
    assert!(
        staged.config_rows("CONFIG").unwrap().is_empty(),
        "CONFIG is the apply's"
    );
    let rows = staged.config_rows("CONFIGSAVE").unwrap();
    for service in ["root", "version", "versions", "deleted"] {
        assert!(rows.iter().any(|row| row.file_name == service), "{service}");
    }
    assert_eq!(staged.verify_index("CONFIGSAVE").unwrap(), rows.len());
    drop(staged);
    // The export publishes a completed stage: the tree comes back, but for
    // the generation ids of ConfigDumpInfo.xml, which every import renews.
    let out = dir.join("xml");
    ok(&export(&db, &out));
    assert_same_tree(&fixtures().join("tiny-xml"), &out, &["ConfigDumpInfo.xml"]);
}

#[test]
fn an_import_onto_the_same_configuration_loads_the_edit() {
    let dir = scratch("import-patch");
    let db = infobase(&dir, "tiny");
    let tree = dir.join("tree");
    for (path, bytes) in files(&fixtures().join("tiny-xml")) {
        let target = tree.join(&path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    let module = tree.join("CommonModules/ФайловаяБазаПроверка/Ext/Module.bsl");
    let text = fs::read(&module).unwrap();
    let edited = String::from_utf8(text)
        .unwrap()
        .replace("Возврат 42;", "Возврат 43;");
    fs::write(&module, edited).unwrap();

    let output = import(&db, &tree);
    ok(&output);
    let report = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(!report.contains("panicked"), "{report}");
    let out = dir.join("xml");
    ok(&export(&db, &out));
    assert_same_tree(&tree, &out, &["ConfigDumpInfo.xml"]);
}

#[test]
fn a_russian_infobase_is_imported_with_its_collation() {
    let dir = scratch("import-ru");
    let db = infobase(&dir, "ru-empty");
    ok(&import(&db, &fixtures().join("tiny-xml")));
    let mut staged = ibcmd_filedb::FileDb::open(&db).unwrap();
    assert_eq!(staged.locale(), "ru_RU");
    staged.verify_index("CONFIGSAVE").unwrap();
    drop(staged);
    let out = dir.join("xml");
    ok(&export(&db, &out));
    assert_same_tree(&fixtures().join("tiny-xml"), &out, &["ConfigDumpInfo.xml"]);
}

#[test]
fn an_infobase_in_an_unmeasured_locale_is_not_imported() {
    let dir = scratch("import-de");
    let db = infobase(&dir, "de-empty");
    let before = fs::read(db.join("1Cv8.1CD")).unwrap();
    let output = import(&db, &fixtures().join("tiny-xml"));
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("de_DE"), "{stderr}");
    assert_eq!(fs::read(db.join("1Cv8.1CD")).unwrap(), before);
    assert_eq!(fs::read_dir(&db).unwrap().count(), 1);
    // Reading it is the crate's test (`native_files.rs`): this infobase holds
    // no configuration, which the export does not publish for any storage.
}

#[test]
fn broken_files_fail_closed() {
    let dir = scratch("broken");
    let db = infobase(&dir, "tiny");
    let data = db.join("1Cv8.1CD");
    let whole = fs::read(&data).unwrap();
    for (tag, bytes) in [
        ("truncated", whole[..whole.len() / 3].to_vec()),
        ("not-a-1cd", b"PK\x03\x04 not an infobase".to_vec()),
        ("huge-object", {
            // The root object's length (page 2, offset 16) far past the file:
            // refused, not allocated (an allocation that size aborts).
            let mut bytes = whole.clone();
            bytes[2 * 8192 + 16..2 * 8192 + 24]
                .copy_from_slice(&0xFFFF_FFFF_FFFF_FF00u64.to_le_bytes());
            bytes
        }),
    ] {
        fs::write(&data, &bytes).unwrap();
        let out = dir.join(format!("xml-{tag}"));
        let output = export(&db, &out);
        assert!(!output.status.success(), "{tag}: the export succeeded");
        assert!(output.status.code().is_some(), "{tag}: killed by a signal");
        assert_ne!(output.status.code(), Some(134), "{tag}: aborted");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("panicked"), "{tag}: {stderr}");
        let output = import(&db, &fixtures().join("tiny-xml"));
        assert!(!output.status.success(), "{tag}: the import succeeded");
        assert_eq!(
            fs::read(&data).unwrap(),
            bytes,
            "{tag}: the file was changed"
        );
        assert!(!ibcmd_filedb::backup_file(&db).exists(), "{tag}");
    }
    // No infobase in the directory at all.
    let output = export(&dir.join("absent"), &dir.join("xml-absent"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("1Cv8.1CD"));
}
