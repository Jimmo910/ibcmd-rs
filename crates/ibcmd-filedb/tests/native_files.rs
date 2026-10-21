//! The reader and the writer against files written by 1C:Enterprise
//! 8.5.1.1150 (`tests/fixtures/file-infobase` of the workspace).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use ibcmd_filedb::{
    BACKUP_FILE, CONFIG_TABLES, ConfigRow, FileDb, Timestamp, replace_config_table,
};

fn unpack(name: &str, tag: &str) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/file-infobase")
        .join(format!("{name}.1CD.gz"));
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(fs::File::open(source).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    let dir = std::env::temp_dir().join(format!("ibcmd-filedb-{tag}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("1Cv8.1CD"), bytes).unwrap();
    dir
}

#[test]
fn native_files_read_and_every_index_matches_its_records() {
    for name in ["empty", "tiny"] {
        let dir = unpack(name, &format!("read-{name}"));
        let mut db = FileDb::open(&dir).unwrap();
        assert_eq!(db.locale(), "en_US_POSIX");
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
    let dir = unpack("tiny", "read-tiny-rows");
    let rows = FileDb::open(&dir).unwrap().config_rows("CONFIG").unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.file_name.as_str()).collect();
    for service in ["root", "version", "versions"] {
        assert!(names.contains(&service), "{service} in {names:?}");
    }
    assert!(rows.iter().all(|r| r.data_size == r.binary.len() as u64));
}

#[test]
fn a_table_written_back_reads_the_same_and_keeps_a_backup() {
    let dir = unpack("tiny", "write-identity");
    let original = fs::read(dir.join("1Cv8.1CD")).unwrap();
    let config = FileDb::open(&dir).unwrap().config_rows("CONFIG").unwrap();
    let report = replace_config_table(&dir, "CONFIGSAVE", &config).unwrap();
    assert_eq!(report.rows_after, config.len());
    assert_eq!(fs::read(dir.join(BACKUP_FILE)).unwrap(), original);
    let mut db = FileDb::open(&dir).unwrap();
    assert_eq!(db.config_rows("CONFIGSAVE").unwrap(), config);
    assert_eq!(db.config_rows("CONFIG").unwrap(), config);
    db.verify_index("CONFIGSAVE").unwrap();
    // Written again over the first write: the space is reused, not grown.
    drop(db);
    let pages = report.pages_after;
    let again = replace_config_table(&dir, "CONFIGSAVE", &config).unwrap();
    assert_eq!(again.pages_after, pages);
}

#[test]
fn a_row_the_index_cannot_key_changes_nothing() {
    let dir = unpack("tiny", "write-refused");
    let before = fs::read(dir.join("1Cv8.1CD")).unwrap();
    let row = ConfigRow {
        file_name: "имя по-русски".into(),
        part_no: 0,
        creation: Timestamp::now_utc(),
        modified: Timestamp::now_utc(),
        attributes: 0,
        data_size: 1,
        binary: vec![0],
    };
    assert!(replace_config_table(&dir, "CONFIGSAVE", &[row]).is_err());
    assert_eq!(fs::read(dir.join("1Cv8.1CD")).unwrap(), before);
    assert!(!dir.join(BACKUP_FILE).exists());
    assert!(
        fs::read_dir(&dir).unwrap().count() == 1,
        "no work file is left"
    );
}
