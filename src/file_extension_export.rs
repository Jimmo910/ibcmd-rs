//! Read-only named extension export from one locked file infobase.
use crate::{
    legacy_version::InfobaseConfigSourceVersion,
    mssql_dump::{
        StorageImageSourceExportReport,
        cas::{CasHash, CasManifest, CasStorageRow, resolve_cas_storage_image},
    },
};
use anyhow::{Context, Result, bail};
use ibcmd_core::storage::{MAX_STORAGE_ENTRIES, MAX_STORAGE_IMAGE_RETAINED_BYTES};
use ibcmd_filedb::{ConfigRow, FileDb};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub(crate) struct MissingExtension(pub String);
impl fmt::Display for MissingExtension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Операция невозможна: расширение {} не найдено", self.0)
    }
}
impl std::error::Error for MissingExtension {}

struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn read_rows(db: &mut FileDb, table: &str) -> Result<Vec<ConfigRow>> {
    Ok(db.config_rows_bounded(
        table,
        MAX_STORAGE_IMAGE_RETAINED_BYTES as u64,
        MAX_STORAGE_ENTRIES,
    )?)
}
fn validate_part(row: &ConfigRow) -> Result<()> {
    if row.part_no != 0 || row.data_size != row.binary.len() as u64 {
        bail!(
            "extension row {:?}: unsupported multipart or inconsistent DataSize",
            row.file_name
        );
    }
    Ok(())
}

pub(crate) fn export(
    db_path: &Path,
    name: &str,
    output: &Path,
    version: InfobaseConfigSourceVersion,
) -> Result<StorageImageSourceExportReport> {
    if version != InfobaseConfigSourceVersion::V2_20 {
        bail!("file extension export supports only the measured 8.3.27 / XML 2.20 profile");
    }
    let mut db = FileDb::open(db_path)?;
    let registry = db.extension_registry()?;
    // Native 8.3.27 accepts Cyrillic case changes and trailing ASCII spaces;
    // it does not require the caller to reproduce the stored spelling.
    let requested = name.trim_end_matches(' ').to_lowercase();
    let mut matches = registry
        .iter()
        .filter(|row| row.name.to_lowercase() == requested);
    let selected = matches
        .next()
        .ok_or_else(|| MissingExtension(name.into()))?;
    if matches.next().is_some() {
        bail!("extension name {name:?} is ambiguous");
    }
    let mut ids = BTreeSet::new();
    if registry.iter().any(|row| !ids.insert(row.physical_id)) {
        bail!("duplicate physical extension registry identity");
    }
    let info = crate::mssql_extensions::decode_file_extension_zipped_info(&selected.zipped_info)
        .with_context(|| format!("invalid file registry envelope for extension {name:?}"))?;
    let prefix = format!(
        "{}__",
        crate::mssql_extension_stage::extension_namespace_prefix(selected.physical_id)
    );
    let mut active = BTreeMap::<CasHash, CasStorageRow>::new();
    for row in read_rows(&mut db, "CONFIGCAS")? {
        validate_part(&row)?;
        let hash = CasHash::parse_hex(&row.file_name)?;
        if active
            .insert(hash, CasStorageRow::new(hash, row.binary))
            .is_some()
        {
            bail!("duplicate active CAS row {hash}");
        }
    }
    let mut staged = Vec::new();
    let mut names = BTreeSet::new();
    for row in read_rows(&mut db, "CONFIGCASSAVE")? {
        let Some(logical) = row.file_name.strip_prefix(&prefix) else {
            continue;
        };
        validate_part(&row)?;
        if !names.insert(logical.to_owned()) {
            bail!("duplicate staged extension row {logical:?}");
        }
        staged.push((logical.to_owned(), i32::try_from(row.part_no)?, row.binary));
    }
    let root = if staged.is_empty() {
        CasHash::parse_hex(&info.active_cas_root)?
    } else {
        let packed_root = staged
            .iter()
            .find(|(name, _, _)| name == "configinfo")
            .context("the staged extension has no configinfo")?;
        let manifest = CasManifest::decode(&packed_root.2)?;
        let expected: BTreeMap<_, _> = manifest
            .entries()
            .iter()
            .map(|entry| (entry.logical_name.as_str(), entry.content_hash))
            .collect();
        for (logical, _, packed) in &staged {
            if logical != "configinfo"
                && expected.get(logical.as_str()) != Some(&CasHash::for_packed_bytes(packed))
            {
                bail!("staged extension row {logical:?} is not classified by configinfo");
            }
        }
        let (root, supplied) = crate::mssql_extension_export::staged_cas_rows(staged)?;
        for row in supplied {
            if let Some(old) = active.get(&row.hash) {
                if old.packed != row.packed {
                    bail!("conflicting active and staged CAS row {}", row.hash);
                }
            } else {
                active.insert(row.hash, row);
            }
        }
        root
    };
    let image = resolve_cas_storage_image(root, active.into_values())?;
    let base = crate::mssql_dump::file_extension_base_provider(&mut db, version)?;
    let stage = Stage(crate::mssql_extension_export::prepare_atomic_staging_root(
        output,
    )?);
    let payload = stage.0.join("payload");
    let mut report = crate::mssql_dump::extension::export_extension_image_to_source(
        &image,
        &payload,
        version,
        Some(base),
    )?;
    if report.storage.failed != 0 || report.storage.opaque != 0 {
        bail!(
            "extension writer is incomplete: {} failed and {} opaque storage rows",
            report.storage.failed,
            report.storage.opaque
        );
    }
    // Removing an existing empty directory is atomic with respect to its
    // contents: a concurrent new file makes remove_dir fail, never gets deleted.
    match fs::symlink_metadata(output) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => fs::remove_dir(output)?,
        Ok(_) => bail!(
            "refusing to replace non-directory or link {}",
            output.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    fs::rename(&payload, output)?;
    report.output_dir = output.to_owned();
    // Keep db (and its read lock) alive through the publication above.
    drop(db);
    Ok(report)
}
