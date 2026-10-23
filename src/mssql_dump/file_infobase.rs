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

/// The configuration a file infobase publishes, as offline rows.
pub(super) fn load(db_path: &Path) -> Result<OfflineRows> {
    let mut db = FileDb::open(db_path)
        .map_err(|err| anyhow!("{err}"))
        .with_context(|| format!("не удалось прочитать файловую базу {}", db_path.display()))?;
    let config = assemble(
        db.config_rows("CONFIG").map_err(|err| anyhow!("{err}"))?,
        "CONFIG",
    )?;
    let saved_names: BTreeSet<String> = db
        .config_headers("CONFIGSAVE")
        .map_err(|err| anyhow!("{err}"))?
        .into_iter()
        .map(|row| row.file_name)
        .collect();
    publish(config, &saved_names, || {
        assemble(
            db.config_rows("CONFIGSAVE")
                .map_err(|err| anyhow!("{err}"))?,
            "CONFIGSAVE",
        )
    })
}

/// The configuration a database publishes from its `CONFIG` rows and the
/// names `CONFIGSAVE` holds: a completed stage (its `versions` row, no
/// marker of an unfinished import or apply, every staged row listed) is
/// published over `CONFIG`, as the platform's export publishes it; anything
/// else leaves `CONFIG` alone. `staged` reads the `CONFIGSAVE` rows, only
/// when they are published.
pub(super) fn publish(
    config: BTreeMap<String, Vec<u8>>,
    saved_names: &BTreeSet<String>,
    staged: impl FnOnce() -> Result<BTreeMap<String, Vec<u8>>>,
) -> Result<OfflineRows> {
    let stored = OfflineRows::from_memory(
        config
            .iter()
            .map(|(name, bytes)| (name.clone(), Arc::new(bytes.clone()))),
    );
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
    let staged = staged()?;
    let versions = staged
        .get(STAGE_INVENTORY_ROW)
        .ok_or_else(|| anyhow!("CONFIGSAVE has a `versions` header but no row"))?;
    // As for a SQL database: a staged row its own `versions` does not list
    // means the stage is not one this reads.
    let published = config.keys().chain(staged.keys()).map(String::as_str);
    match super::config_dump_info::unlisted_entries(versions, published) {
        Ok(unlisted) if unlisted.is_disjoint(saved_names) => stored.with_staged(
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

/// Resolve adopted references from the same locked descriptor as the
/// extension registry/CAS. No second open may drop its POSIX record lock.
pub(crate) fn file_extension_base_provider(
    db: &mut FileDb,
    version: crate::legacy_version::InfobaseConfigSourceVersion,
) -> Result<super::extension::BaseIndexProvider> {
    let rows = db.config_rows_bounded(
        "CONFIG",
        ibcmd_core::storage::MAX_STORAGE_IMAGE_RETAINED_BYTES as u64,
        ibcmd_core::storage::MAX_STORAGE_ENTRIES,
    )?;
    let config = file_infobase_config(rows)?;
    let rows = config
        .published
        .into_iter()
        .filter(|(name, _)| !name.contains('.'))
        .map(|(file_name, bytes)| {
            Ok(super::ConfigRow {
                file_name,
                part_no: 0,
                data_size: i64::try_from(bytes.len())?,
                #[cfg(test)]
                binary_hex: super::encode_hex_lower(&bytes),
                #[cfg(not(test))]
                binary_hex: String::new(),
                #[cfg(not(test))]
                binary: Some(bytes),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let indexes = file_extension_base_indexes(&rows, version)?;
    Ok(Arc::new(move || Some(indexes.clone())))
}

fn file_extension_base_indexes(
    rows: &[super::ConfigRow],
    version: crate::legacy_version::InfobaseConfigSourceVersion,
) -> Result<super::extension::ResolvedIndexes> {
    check_file_base_metadata_budget(rows, 64 * 1024 * 1024, 256 * 1024 * 1024)?;
    Ok(super::extension::base_indexes_from_rows(rows, version))
}

// The index builder inflates this immutable input. Count its output with a
// fixed buffer first, bounding each allocation and all retained metadata text.
fn check_file_base_metadata_budget(
    rows: &[super::ConfigRow],
    per_row: usize,
    aggregate: usize,
) -> Result<()> {
    use std::io::Read;
    let mut total = 0usize;
    let mut buffer = [0u8; 16 * 1024];
    for row in rows.iter().filter(|row| !row.file_name.contains('.')) {
        let packed = row.binary_bytes()?;
        let allowed = per_row.min(aggregate.saturating_sub(total));
        let mut decoder =
            flate2::read::DeflateDecoder::new(packed.as_ref()).take(allowed as u64 + 1);
        let mut count = 0usize;
        loop {
            let read = match decoder.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => read,
            };
            count += read;
            if count > allowed {
                bail!(
                    "file base metadata {:?} exceeds the inflation budget",
                    row.file_name
                );
            }
        }
        total += count;
    }
    Ok(())
}

#[cfg(test)]
mod extension_base_budget_tests {
    use super::*;
    use std::io::Read;

    fn row(name: &str, bytes: &[u8]) -> super::super::ConfigRow {
        let packed = crate::module_blob::deflate_raw(bytes).unwrap();
        super::super::ConfigRow {
            file_name: name.into(),
            part_no: 0,
            data_size: packed.len() as i64,
            binary_hex: super::super::encode_hex_lower(&packed),
        }
    }

    #[test]
    fn file_base_metadata_budgets_count_each_row_and_the_aggregate() {
        let rows = [row("first", &[b'x'; 6]), row("second", &[b'y'; 6])];
        assert!(check_file_base_metadata_budget(&rows, 6, 12).is_ok());
        assert!(check_file_base_metadata_budget(&rows, 5, 12).is_err());
        assert!(check_file_base_metadata_budget(&rows, 6, 11).is_err());
    }

    #[test]
    fn file_extension_base_index_refuses_oversize_inflation_before_indexing() {
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::copy(
            &mut std::io::repeat(b'x').take(64 * 1024 * 1024 + 1),
            &mut encoder,
        )
        .unwrap();
        let packed = encoder.finish().unwrap();
        let rows = [super::super::ConfigRow {
            file_name: "metadata".into(),
            part_no: 0,
            data_size: packed.len() as i64,
            binary_hex: super::super::encode_hex_lower(&packed),
        }];
        assert!(
            file_extension_base_indexes(
                &rows,
                crate::legacy_version::InfobaseConfigSourceVersion::V2_20
            )
            .is_err(),
            "oversize inflated file metadata was accepted by the base index provider"
        );
    }
}

/// Stored rows by name, each assembled from its parts in order.
fn assemble(rows: Vec<ConfigRow>, table: &str) -> Result<BTreeMap<String, Vec<u8>>> {
    assemble_parts(
        rows.into_iter()
            .map(|row| (row.file_name, row.part_no, row.binary)),
        table,
    )
}

/// The rows of a configuration table by name, each with its parts joined in
/// part order; a part stored twice or a gap in the parts is refused.
pub(super) fn assemble_parts(
    rows: impl IntoIterator<Item = (String, u32, Vec<u8>)>,
    table: &str,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut parts = BTreeMap::<String, BTreeMap<u32, Vec<u8>>>::new();
    for (file_name, part_no, binary) in rows {
        if parts
            .entry(file_name.clone())
            .or_default()
            .insert(part_no, binary)
            .is_some()
        {
            bail!("{table}: строка {file_name} часть {part_no} записана дважды");
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
