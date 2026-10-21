//! The reader and the writer against files written by 1C:Enterprise
//! 8.5.1.1150 (`tests/fixtures/file-infobase` of the workspace).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use ibcmd_filedb::{
    CONFIG_TABLES, Collation, ConfigRow, Error, FileDb, Timestamp, backup_file, open_for_write,
    replace_config_table,
};

/// A scratch directory holding one unpacked fixture, removed on drop.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn unpack(name: &str) -> Scratch {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/file-infobase")
        .join(format!("{name}.1CD.gz"));
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(fs::File::open(source).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    let dir = std::env::temp_dir().join(format!(
        "ibcmd-filedb-{}-{}-{name}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("1Cv8.1CD"), bytes).unwrap();
    Scratch(dir)
}

fn row(name: &str, part_no: u32, binary: Vec<u8>) -> ConfigRow {
    ConfigRow {
        file_name: name.into(),
        part_no,
        creation: Timestamp::now_utc(),
        modified: Timestamp::now_utc(),
        attributes: 0,
        data_size: binary.len() as u64,
        binary,
    }
}

#[test]
fn native_files_read_and_every_index_matches_its_records() {
    for (name, locale) in [
        ("empty", "en_US_POSIX"),
        ("tiny", "en_US_POSIX"),
        ("ru-empty", "ru_RU"),
    ] {
        let dir = unpack(name);
        let mut db = FileDb::open(&dir.0).unwrap();
        assert_eq!(db.locale(), locale);
        assert_eq!(db.page_size(), 8192);
        for table in CONFIG_TABLES {
            let rows = db.config_rows(table).unwrap();
            assert_eq!(
                db.verify_index(table).unwrap(),
                rows.len(),
                "{name} {table}"
            );
        }
    }
    let dir = unpack("tiny");
    let rows = FileDb::open(&dir.0).unwrap().config_rows("CONFIG").unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.file_name.as_str()).collect();
    for service in ["root", "version", "versions"] {
        assert!(names.contains(&service), "{service} in {names:?}");
    }
    assert!(rows.iter().all(|r| r.data_size == r.binary.len() as u64));
}

#[test]
fn a_table_written_back_reads_the_same_and_keeps_a_backup() {
    let dir = unpack("tiny");
    let data = dir.0.join("1Cv8.1CD");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&data, fs::Permissions::from_mode(0o640)).unwrap();
    }
    let original = fs::read(&data).unwrap();
    let config = FileDb::open(&dir.0).unwrap().config_rows("CONFIG").unwrap();
    let report = replace_config_table(&dir.0, "CONFIGSAVE", &config).unwrap();
    assert_eq!(report.rows_after, config.len());
    assert_eq!(report.backup, backup_file(&dir.0));
    assert_eq!(fs::read(backup_file(&dir.0)).unwrap(), original);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&data).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640, "the data file keeps its mode");
    }
    let mut db = FileDb::open(&dir.0).unwrap();
    assert_eq!(db.config_rows("CONFIGSAVE").unwrap(), config);
    assert_eq!(db.config_rows("CONFIG").unwrap(), config);
    db.verify_index("CONFIGSAVE").unwrap();
    drop(db);
    // Written again over the first write: the space is reused, not grown.
    let again = replace_config_table(&dir.0, "CONFIGSAVE", &config).unwrap();
    assert_eq!(again.pages_after, report.pages_after);
}

#[test]
fn objects_grow_to_second_level_tables_and_shrink_onto_free_chains() {
    let dir = unpack("tiny");
    let mut rows: Vec<ConfigRow> = (0..6000)
        .map(|i| {
            row(
                &format!("{i:08x}-0000-4000-8000-{:012x}.{}", i * 7919, i % 3),
                0,
                vec![(i % 251) as u8; 40 + i % 700],
            )
        })
        .collect();
    // Three rows of three parts each, as long rows are stored.
    for n in 0..3u8 {
        for part in 0..3 {
            rows.push(row(&format!("big{n}.0"), part, vec![n; 4_000_000]));
        }
    }
    let big = replace_config_table(&dir.0, "CONFIGSAVE", &rows).unwrap();
    assert!(big.pages_after > 1500, "{}", big.pages_after);
    let mut db = FileDb::open(&dir.0).unwrap();
    assert_eq!(db.config_rows("CONFIGSAVE").unwrap(), rows);
    assert_eq!(db.verify_index("CONFIGSAVE").unwrap(), rows.len());
    db.verify_index("CONFIG").unwrap();
    drop(db);
    let few = &rows[..10];
    let small = replace_config_table(&dir.0, "CONFIGSAVE", few).unwrap();
    assert_eq!(small.pages_after, big.pages_after, "objects never shrink");
    let mut db = FileDb::open(&dir.0).unwrap();
    assert_eq!(db.config_rows("CONFIGSAVE").unwrap(), few);
    db.verify_index("CONFIGSAVE").unwrap();
}

#[test]
fn a_russian_infobase_is_written_with_its_own_collation() {
    let dir = unpack("ru-empty");
    let session = open_for_write(&dir.0).unwrap();
    assert_eq!(session.collation(), Collation::Russian);
    let rows = vec![
        row("root", 0, vec![1]),
        row("Versions", 0, vec![2]),
        row("a2782760-dd4c-44d2-a785-814d6f610455.0", 0, vec![3]),
    ];
    session.replace("CONFIGSAVE", &rows).unwrap();
    let mut db = FileDb::open(&dir.0).unwrap();
    assert_eq!(db.verify_index("CONFIGSAVE").unwrap(), 3);
    db.verify_index("PARAMS").unwrap();
}

#[test]
fn an_infobase_in_an_unmeasured_locale_is_read_but_not_written() {
    let dir = unpack("de-empty");
    let mut db = FileDb::open(&dir.0).unwrap();
    assert_eq!(db.locale(), "de_DE");
    assert_eq!(db.collation(), None);
    assert!(!db.config_rows("PARAMS").unwrap().is_empty());
    drop(db);
    let before = fs::read(dir.0.join("1Cv8.1CD")).unwrap();
    let refused = replace_config_table(&dir.0, "CONFIGSAVE", &[row("root", 0, vec![1])]);
    assert!(matches!(refused, Err(Error::Refused(_))), "{refused:?}");
    assert_eq!(fs::read(dir.0.join("1Cv8.1CD")).unwrap(), before);
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
}

#[test]
fn a_row_the_index_cannot_key_changes_nothing() {
    let dir = unpack("tiny");
    let before = fs::read(dir.0.join("1Cv8.1CD")).unwrap();
    let refused = replace_config_table(&dir.0, "CONFIGSAVE", &[row("имя по-русски", 0, vec![0])]);
    assert!(refused.is_err());
    assert_eq!(fs::read(dir.0.join("1Cv8.1CD")).unwrap(), before);
    assert_eq!(
        fs::read_dir(&dir.0).unwrap().count(),
        1,
        "no work file, no backup"
    );
}

#[test]
fn a_work_file_left_behind_stops_the_write() {
    let dir = unpack("tiny");
    let work = dir.0.join("1Cv8.1CD.ibcmd-rs.tmp");
    fs::write(&work, b"left by an interrupted write").unwrap();
    let refused = replace_config_table(&dir.0, "CONFIGSAVE", &[row("root", 0, vec![1])]);
    assert!(matches!(refused, Err(Error::Refused(_))), "{refused:?}");
    assert_eq!(fs::read(&work).unwrap(), b"left by an interrupted write");
}

#[test]
fn a_corrupted_object_length_is_refused_without_allocating_it() {
    let dir = unpack("tiny");
    let data = dir.0.join("1Cv8.1CD");
    let mut bytes = fs::read(&data).unwrap();
    // The root object's length (page 2, offset 16).
    bytes[2 * 8192 + 16..2 * 8192 + 24].copy_from_slice(&0xFFFF_FFFF_FFFF_FF00u64.to_le_bytes());
    fs::write(&data, &bytes).unwrap();
    assert!(matches!(FileDb::open(&dir.0), Err(Error::Corrupt(_))));
    assert!(open_for_write(&dir.0).is_err());
}
