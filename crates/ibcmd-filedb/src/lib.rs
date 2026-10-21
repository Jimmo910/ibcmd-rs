//! File infobases (`1Cv8.1CD`, format 8.3.8) without the platform.
//!
//! Reading: [`FileDb::open`] and [`FileDb::config_rows`] give the rows of the
//! configuration tables (`CONFIG`, `CONFIGSAVE`, `PARAMS`, …).
//!
//! Writing: [`replace_config_table`] replaces every row of one configuration
//! table (`CONFIGSAVE` for an import). The infobase must not be open; the
//! change is made in a copy, read back and checked, the original is kept as
//! `1Cv8.1CD.ibcmd-rs.bak` and the copy takes its place by a rename.
//!
//! The format evidence (page, object, blob, record and index layouts measured
//! on files written by 1C:Enterprise 8.5.1.1150) is in
//! `openspec/changes/add-file-infobase-backend/evidence/1cd-format.md`.

#![deny(unsafe_code)]

mod blob;
mod config;
mod descr;
mod error;
pub mod index;
mod lock;
mod object;
mod pager;
mod record;

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub use config::ConfigRow;
pub use descr::{Field, FieldKind, Table};
pub use error::{Error, Result};
pub use record::Timestamp;

use lock::Mode;
use pager::Pager;

/// The data file inside an infobase directory.
pub const DATA_FILE: &str = "1Cv8.1CD";
/// The copy of the data file kept before the last write.
pub const BACKUP_FILE: &str = "1Cv8.1CD.ibcmd-rs.bak";
const WORK_FILE: &str = "1Cv8.1CD.ibcmd-rs.tmp";

/// The tables a write may replace.
pub const CONFIG_TABLES: [&str; 6] = [
    "CONFIG",
    "CONFIGSAVE",
    "PARAMS",
    "FILES",
    "CONFIGCAS",
    "CONFIGCASSAVE",
];

/// Resolves `--db-path`: the directory holding `1Cv8.1CD`, or the file itself
/// when the path names a `.1CD` file.
pub fn data_file(db_path: &Path) -> PathBuf {
    let names_file = !db_path.is_dir()
        && db_path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("1cd"));
    if names_file {
        db_path.to_path_buf()
    } else {
        db_path.join(DATA_FILE)
    }
}

/// An open file infobase, read-locked for the lifetime of the value.
pub struct FileDb {
    pager: Pager,
    locale: String,
    tables: BTreeMap<String, Table>,
}

impl FileDb {
    /// Opens `1Cv8.1CD` (or the directory holding it) for reading.
    pub fn open(db_path: &Path) -> Result<FileDb> {
        let path = data_file(db_path);
        if !path.is_file() {
            return Err(Error::Refused(format!(
                "Файловая информационная база не найдена: нет файла {}",
                path.display()
            )));
        }
        let file = lock::open_locked(&path, Mode::Read)?;
        FileDb::from_file(file)
    }

    fn from_file(file: File) -> Result<FileDb> {
        let mut pager = Pager::new(file)?;
        let (locale, tables) = descr::read_tables(&mut pager)?;
        Ok(FileDb {
            pager,
            locale,
            tables,
        })
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn page_size(&self) -> usize {
        self.pager.page_size()
    }

    pub fn page_count(&self) -> u32 {
        self.pager.page_count()
    }

    pub fn tables(&self) -> impl Iterator<Item = &Table> {
        self.tables.values()
    }

    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }

    fn config_table(&self, name: &str) -> Result<Table> {
        let table = self
            .tables
            .get(name)
            .ok_or_else(|| Error::Refused(format!("В базе нет таблицы {name}")))?;
        config::check_layout(table)?;
        Ok(table.clone())
    }

    /// Every stored part of every row of a configuration table, in record
    /// order. Parts of one row are separate values with the same name.
    pub fn config_rows(&mut self, table: &str) -> Result<Vec<ConfigRow>> {
        let table = self.config_table(table)?;
        Ok(config::read_records(&mut self.pager, &table, true)?
            .into_iter()
            .map(|r| r.row)
            .collect())
    }

    /// Row headers without the binary data.
    pub fn config_headers(&mut self, table: &str) -> Result<Vec<ConfigRow>> {
        let table = self.config_table(table)?;
        Ok(config::read_records(&mut self.pager, &table, false)?
            .into_iter()
            .map(|r| r.row)
            .collect())
    }

    /// `(record number, key)` of every index entry, in key order.
    pub fn index_entries(&mut self, table: &str) -> Result<Vec<(u32, Vec<u8>)>> {
        let table = self.config_table(table)?;
        let object = object::Object::open(&mut self.pager, table.index_page)?;
        let data = object.read_all(&mut self.pager)?;
        index::read_entries(&data, self.pager.page_size())
    }

    /// Checks that the index of a configuration table lists exactly the live
    /// records with the keys their names and part numbers give.
    pub fn verify_index(&mut self, table: &str) -> Result<usize> {
        let t = self.config_table(table)?;
        let records = config::read_records(&mut self.pager, &t, false)?;
        let mut expected: Vec<(u32, Vec<u8>)> = records
            .iter()
            .map(|r| {
                Ok((
                    r.number,
                    index::config_key(&r.row.file_name, r.row.part_no)?,
                ))
            })
            .collect::<Result<_>>()?;
        expected.sort_by(|a, b| a.1.cmp(&b.1));
        let actual = self.index_entries(table)?;
        if actual != expected {
            let first = actual
                .iter()
                .zip(&expected)
                .position(|(a, b)| a != b)
                .unwrap_or(actual.len().min(expected.len()));
            return Err(Error::Corrupt(format!(
                "Индекс таблицы {table} не совпадает с записями: {} ключей против {} записей, первое расхождение в позиции {first}",
                actual.len(),
                expected.len()
            )));
        }
        Ok(actual.len())
    }
}

/// Fails with [`Error::Busy`] when the platform or another process has the
/// infobase open, so a write is refused before any work is done for it. The
/// probe lock is released at once.
pub fn ensure_closed(db_path: &Path) -> Result<()> {
    let path = data_file(db_path);
    drop(lock::open_locked(&path, Mode::Write)?);
    Ok(())
}

/// What a write did.
#[derive(Debug, Clone)]
pub struct ReplaceReport {
    pub table: String,
    pub rows_before: usize,
    pub rows_after: usize,
    pub bytes_after: u64,
    pub pages_before: u32,
    pub pages_after: u32,
    pub backup: PathBuf,
}

/// Replaces every row of a configuration table of a closed file infobase.
///
/// The steps: lock the data file exclusively, copy it to a work file, write
/// the table into the copy, read the copy back and compare every row and the
/// index with `rows`, keep the original as [`BACKUP_FILE`] (a hard link when
/// the file system allows it), rename the copy over the data file. A failure
/// before the rename leaves the infobase untouched.
pub fn replace_config_table(
    db_path: &Path,
    table: &str,
    rows: &[ConfigRow],
) -> Result<ReplaceReport> {
    if !CONFIG_TABLES.contains(&table) {
        return Err(Error::Refused(format!(
            "Запись таблицы {table} не поддерживается"
        )));
    }
    let path = data_file(db_path);
    let dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut original = lock::open_locked(&path, Mode::Write)?;
    let work = dir.join(WORK_FILE);
    let backup = dir.join(BACKUP_FILE);
    let result = (|| -> Result<ReplaceReport> {
        let _ = fs::remove_file(&work);
        let mut copy = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&work)?;
        original.seek(SeekFrom::Start(0))?;
        std::io::copy(&mut original, &mut copy)?;
        let mut pager = Pager::new(copy)?;
        let pages_before = pager.page_count();
        let (_, tables) = descr::read_tables(&mut pager)?;
        let t = tables
            .get(table)
            .ok_or_else(|| Error::Refused(format!("В базе нет таблицы {table}")))?
            .clone();
        let rows_before = config::read_records(&mut pager, &t, false)?.len();
        config::replace(&mut pager, &t, rows)?;
        pager.sync()?;
        let pages_after = pager.page_count();
        drop(pager);

        // Read the copy back as an independent reader would.
        let check = OpenOptions::new().read(true).open(&work)?;
        let mut db = FileDb::from_file(check)?;
        let written = db.config_rows(table)?;
        if written.len() != rows.len() || written.iter().zip(rows).any(|(a, b)| a != b) {
            return Err(Error::Corrupt(format!(
                "Проверка записи: таблица {table} прочиталась не так, как записана"
            )));
        }
        db.verify_index(table)?;
        drop(db);

        let _ = fs::remove_file(&backup);
        if fs::hard_link(&path, &backup).is_err() {
            original.seek(SeekFrom::Start(0))?;
            let mut out = File::create(&backup)?;
            std::io::copy(&mut original, &mut out)?;
            out.sync_all()?;
        }
        Ok(ReplaceReport {
            table: table.to_string(),
            rows_before,
            rows_after: rows.len(),
            bytes_after: rows.iter().map(|r| r.binary.len() as u64).sum(),
            pages_before,
            pages_after,
            backup: backup.clone(),
        })
    })();
    let report = match result {
        Ok(report) => report,
        Err(err) => {
            let _ = fs::remove_file(&work);
            return Err(err);
        }
    };
    // Windows cannot replace a file that is open; the exclusive handle goes
    // first there. On Unix the lock is held across the rename.
    #[cfg(windows)]
    drop(original);
    if let Err(err) = fs::rename(&work, &path) {
        let _ = fs::remove_file(&work);
        return Err(Error::Io(err));
    }
    #[cfg(unix)]
    {
        if let Ok(d) = File::open(&dir) {
            let _ = d.sync_all();
        }
        drop(original);
    }
    Ok(report)
}

/// Research aid: rebuilds the index of one configuration table in place, from
/// its live records. The infobase must be closed; no backup is made.
#[doc(hidden)]
pub fn rebuild_config_index_in_place(db_path: &Path, table: &str) -> Result<()> {
    let path = data_file(db_path);
    let file = lock::open_locked(&path, Mode::Write)?;
    let mut pager = Pager::new(file)?;
    let (_, tables) = descr::read_tables(&mut pager)?;
    let t = tables
        .get(table)
        .ok_or_else(|| Error::Refused(format!("В базе нет таблицы {table}")))?
        .clone();
    config::reindex(&mut pager, &t)?;
    pager.sync()
}
