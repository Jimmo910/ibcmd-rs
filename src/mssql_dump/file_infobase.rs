//! The export of a file infobase (`--db-path`, #373): the rows of its
//! `CONFIG` table, with a completed import stage of `CONFIGSAVE` folded in the
//! way the platform's export publishes it, become the offline rows that every
//! read of the export answers from. The 1Cv8.1CD format itself is
//! `ibcmd-filedb`'s.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use ibcmd_filedb::{ConfigRow, FileDb};

use super::offline_rows::OfflineRows;
use super::{STAGE_COMMIT_ROW, STAGE_INVENTORY_ROW, STAGE_NEW_SUFFIX};

/// The `CONFIG` table of a file infobase alone, as offline rows: the
/// database configuration (`config save --db`).
pub(super) fn load_stored(db_path: &Path) -> Result<OfflineRows> {
    let mut db = open(db_path)?;
    let config = assemble(
        db.config_rows("CONFIG").map_err(|err| anyhow!("{err}"))?,
        "CONFIG",
    )?;
    Ok(OfflineRows::from_memory(
        config
            .into_iter()
            .map(|(name, bytes)| (name, Arc::new(bytes))),
    ))
}

fn open(db_path: &Path) -> Result<FileDb> {
    FileDb::open(db_path)
        .map_err(|err| anyhow!("{err}"))
        .with_context(|| format!("не удалось прочитать файловую базу {}", db_path.display()))
}

/// The configuration a file infobase publishes, as offline rows.
pub(super) fn load(db_path: &Path) -> Result<OfflineRows> {
    Ok(load_with_stored_names(db_path)?.0)
}

/// Retains the original CONFIG inventory separately from the published view:
/// a staged `deleted` row is not an unfinished operation in stored CONFIG.
pub(super) fn load_with_stored_names(db_path: &Path) -> Result<(OfflineRows, BTreeSet<String>)> {
    let mut db = open(db_path)?;
    let config = assemble(
        db.config_rows("CONFIG").map_err(|err| anyhow!("{err}"))?,
        "CONFIG",
    )?;
    let stored_names = config.keys().cloned().collect();
    let published = published_rows(&mut db, config)?;
    Ok((published, stored_names))
}

fn published_rows(db: &mut FileDb, config: BTreeMap<String, Vec<u8>>) -> Result<OfflineRows> {
    let stored = OfflineRows::from_memory(
        config
            .iter()
            .map(|(name, bytes)| (name.clone(), Arc::new(bytes.clone()))),
    );
    let saved_names: BTreeSet<String> = db
        .config_headers("CONFIGSAVE")
        .map_err(|err| anyhow!("{err}"))?
        .into_iter()
        .map(|row| row.file_name)
        .collect();
    if saved_names.is_empty() {
        return Ok(stored);
    }
    let unfinished = saved_names.contains(STAGE_COMMIT_ROW)
        || saved_names
            .iter()
            .any(|name| name.ends_with(STAGE_NEW_SUFFIX));
    if !saved_names.contains(STAGE_INVENTORY_ROW) || unfinished {
        eprintln!(
            "CONFIGSAVE holds no complete stage (no `versions` row, or the markers of an unfinished import or apply); the export publishes the CONFIG table"
        );
        return Ok(stored);
    }
    let staged = assemble(
        db.config_rows("CONFIGSAVE")
            .map_err(|err| anyhow!("{err}"))?,
        "CONFIGSAVE",
    )?;
    let versions = staged
        .get(STAGE_INVENTORY_ROW)
        .ok_or_else(|| anyhow!("CONFIGSAVE has a `versions` header but no row"))?;
    // As for a SQL database: a staged row its own `versions` does not list
    // means the stage is not one this reads.
    let published = config.keys().chain(staged.keys()).map(String::as_str);
    match super::config_dump_info::unlisted_entries(versions, published) {
        Ok(unlisted) if unlisted.is_disjoint(&saved_names) => stored.with_staged(
            staged
                .into_iter()
                .map(|(name, bytes)| (name, Arc::new(bytes))),
            &[],
        ),
        Ok(_) => {
            eprintln!(
                "CONFIGSAVE holds rows its `versions` does not list; the export publishes the CONFIG table"
            );
            Ok(stored)
        }
        Err(error) => {
            eprintln!(
                "CONFIGSAVE `versions` cannot be read ({error:#}); the export publishes the CONFIG table"
            );
            Ok(stored)
        }
    }
}

/// The `CONFIG` table of a file infobase, the base an import stages over.
pub(crate) struct FileInfobaseConfig {
    /// The configuration it publishes (an online generation's aliases under
    /// their plain names): name -> stored bytes.
    pub published: BTreeMap<String, Vec<u8>>,
    /// Every row name it stores.
    pub stored_names: BTreeSet<String>,
}

/// The `CONFIG` rows of a file infobase as an import stages over them; a table
/// holding the markers of an operation the platform did not finish is
/// refused, as `config save` refuses it.
pub(crate) fn file_infobase_config(rows: Vec<ConfigRow>) -> Result<FileInfobaseConfig> {
    let config = assemble(rows, "CONFIG")?;
    super::config_save::refuse_unfinished_operation(config.keys().map(String::as_str))?;
    let stored_names = config.keys().cloned().collect();
    let published = OfflineRows::from_memory(
        config
            .into_iter()
            .map(|(name, bytes)| (name, Arc::new(bytes))),
    )
    .with_staged(Vec::<(String, Arc<Vec<u8>>)>::new(), &[])?
    .into_rows()?;
    Ok(FileInfobaseConfig {
        published,
        stored_names,
    })
}

/// Stored rows by name, each assembled from its parts in order.
fn assemble(rows: Vec<ConfigRow>, table: &str) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut parts = BTreeMap::<String, BTreeMap<u32, Vec<u8>>>::new();
    for row in rows {
        if parts
            .entry(row.file_name.clone())
            .or_default()
            .insert(row.part_no, row.binary)
            .is_some()
        {
            bail!(
                "{table}: строка {} часть {} записана дважды",
                row.file_name,
                row.part_no
            );
        }
    }
    parts
        .into_iter()
        .map(|(name, parts)| {
            for (expected, part_no) in parts.keys().enumerate() {
                if *part_no as usize != expected {
                    bail!("{table}: у строки {name} часть {part_no} на месте части {expected}");
                }
            }
            Ok((name, parts.into_values().flatten().collect()))
        })
        .collect()
}
