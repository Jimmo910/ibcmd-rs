//! Read-only named extension export from the measured 8.3.27 file profile.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::infobase::{ConnectionConfig, InfobaseConfigExportReport, absolute_path};
use crate::mssql_dump::cas::{
    CasHash, CasManifest, CasStorageRow, resolve_cas_storage_image_with_origin,
};
use anyhow::{Context, Result, anyhow, bail};
use ibcmd_filedb::{ConfigRow, ExtensionRow, FileDb};

pub(crate) fn export(
    config: &ConnectionConfig,
    name: &str,
    output: &Path,
) -> Result<InfobaseConfigExportReport> {
    let source_version = config.legacy_source_version()?;
    if source_version != crate::legacy_version::InfobaseConfigSourceVersion::V2_20 {
        bail!(
            "Файловый экспорт расширения поддерживает только измеренный XML-профиль 8.3.27 (2.20)"
        );
    }
    let output = absolute_path(output)?;
    check_output(&output)?;
    let path = config
        .file_infobase
        .as_deref()
        .ok_or_else(|| anyhow!("no file infobase"))?;
    let mut db = FileDb::open(path).map_err(|e| anyhow!("{e}"))?;
    let registry = db.extension_registry().map_err(|e| anyhow!("{e}"))?;
    let selected = select(&registry, name)?;
    let envelope =
        crate::mssql_extensions::decode_file_extension_zipped_info(&selected.zipped_info)?;
    let root = CasHash::parse_hex(&envelope.active_cas_root)?;
    if !db
        .config_headers_bounded("CONFIGCASSAVE")
        .map_err(|e| anyhow!("{e}"))?
        .is_empty()
        || !db
            .config_headers_bounded("CONFIGSAVE")
            .map_err(|e| anyhow!("{e}"))?
            .is_empty()
    {
        bail!("Подготовленное состояние файловой базы не поддерживается: экспорт не опубликован");
    }
    let root_name = root.to_hex();
    let root_rows = assemble(
        db.config_rows_named("CONFIGCAS", &BTreeSet::from([root_name.clone()]))
            .map_err(|e| anyhow!("{e}"))?,
    )?;
    let packed = root_rows
        .get(&root_name)
        .ok_or_else(|| anyhow!("CAS расширения: корень отсутствует"))?;
    if CasHash::for_packed_bytes(packed) != root {
        bail!("CAS расширения: контрольная сумма корня не совпадает");
    }
    let manifest = CasManifest::decode(packed)?;
    if manifest.identity().storage_format != 80_327 {
        bail!(
            "Неподдержанный файловый формат расширения {}: требуется измеренный 80327",
            manifest.identity().storage_format
        );
    }
    let names = manifest
        .referenced_hashes()
        .iter()
        .map(|h| h.to_hex())
        .collect();
    let mut content = assemble(
        db.config_rows_named("CONFIGCAS", &names)
            .map_err(|e| anyhow!("{e}"))?,
    )?;
    content.extend(root_rows);
    let rows = content
        .into_iter()
        .map(|(name, packed)| Ok(CasStorageRow::new(CasHash::parse_hex(&name)?, packed)))
        .collect::<Result<Vec<_>>>()?;
    let origin = ibcmd_core::storage::StorageOrigin::new(
        ibcmd_core::artifact::StorageProfileId::parse("storage:file-configcas")?,
        ibcmd_core::storage::StorageProvenance::new(&format!("file:configcas-root:{root}"))?,
    );
    let image = resolve_cas_storage_image_with_origin(root, rows, origin)?;
    // Base indexes come from the same held file read lock, never SQL or a
    // different extension. An unfinished main configuration is refused.
    let base = crate::mssql_dump::file_infobase_config(
        db.config_rows_bounded("CONFIG")
            .map_err(|e| anyhow!("{e}"))?,
    )?;
    let provider =
        crate::mssql_dump::extension::file_base_index_provider(base.published, source_version)?;
    let parent = output
        .parent()
        .ok_or_else(|| anyhow!("output has no parent"))?;
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(".ibcmd-rs-file-extension-{}", uuid::Uuid::new_v4()));
    let result = crate::mssql_dump::extension::export_extension_image_to_source(
        &image,
        &staging,
        source_version,
        Some(provider),
    )
    .with_context(|| format!("не удалось выгрузить расширение {name:?}"));
    let export = match result {
        Ok(export) if export.storage.failed == 0 && export.storage.opaque == 0 => export,
        Ok(_) => bail!(
            "Расширение {name:?} не представлено точно: output не опубликован; staging {}",
            staging.display()
        ),
        Err(error) => return Err(error),
    };
    check_output(&output)?;
    if output.exists() {
        fs::remove_dir(&output)?;
    }
    fs::rename(&staging, &output)?;
    Ok(InfobaseConfigExportReport {
        operation: "infobase config export",
        backend: "file-extension-direct",
        format: "xml",
        source_version: source_version.as_str(),
        dbms: config.dbms.clone(),
        db_server: config.db_server.clone(),
        db_name: config.db_name.clone(),
        db_user: None,
        password_source: config.password_source.clone(),
        native_config: config.native_config.clone(),
        temp_dump_dir: output.clone(),
        output_dir: output,
        exported_files: Some(export.files_written),
        raw_rows: export.storage.physical_entries,
        metadata_xml_rows: 0,
        module_text_rows: 0,
        source_asset_rows: 0,
        dump_timings: Default::default(),
        incremental: None,
    })
}

fn select<'a>(rows: &'a [ExtensionRow], name: &str) -> Result<&'a ExtensionRow> {
    let mut found = rows.iter().filter(|r| r.name == name);
    let row = found
        .next()
        .ok_or_else(|| anyhow!("Расширение конфигурации {name:?} не найдено"))?;
    if found.next().is_some() {
        bail!("Имя расширения {name:?} неоднозначно");
    }
    Ok(row)
}

fn check_output(output: &Path) -> Result<()> {
    match fs::symlink_metadata(output) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {
            if fs::read_dir(output)?.next().is_some() {
                bail!("Каталог результата не пуст: {}", output.display());
            }
        }
        Ok(_) => bail!("Результат должен быть каталогом, без символических ссылок"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

fn assemble(rows: Vec<ConfigRow>) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut groups = BTreeMap::<String, BTreeMap<u32, ConfigRow>>::new();
    for row in rows {
        let name = row.file_name.clone();
        let part = row.part_no;
        if row.attributes != 0 || row.data_size > 512 * 1024 * 1024 {
            bail!("Неподдержанный профиль CAS-строки {name}");
        }
        if groups.entry(name).or_default().insert(part, row).is_some() {
            bail!("Повторная часть CAS-строки");
        }
    }
    groups
        .into_iter()
        .map(|(name, parts)| {
            let mut bytes = Vec::new();
            let mut size = None;
            for (expected, (part, row)) in parts.into_iter().enumerate() {
                if expected as u32 != part || size.is_some_and(|n| n != row.data_size) {
                    bail!("Неверные части CAS-строки {name}");
                }
                size = Some(row.data_size);
                bytes.extend(row.binary);
                if bytes.len() > 512 * 1024 * 1024 {
                    bail!("CAS-строка слишком велика");
                }
            }
            if Some(bytes.len() as u64) != size {
                bail!("Размер CAS-строки {name} не совпадает с данными");
            }
            Ok((name, bytes))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_exactly_one_name() {
        let rows = [
            ExtensionRow {
                physical_id: [1; 16],
                name: "One".into(),
                zipped_info: vec![],
            },
            ExtensionRow {
                physical_id: [2; 16],
                name: "Two".into(),
                zipped_info: vec![],
            },
        ];
        assert_eq!(select(&rows, "Two").unwrap().physical_id, [2; 16]);
        assert!(select(&rows, "Unknown").is_err());
        assert!(select(&[rows[0].clone(), rows[0].clone()], "One").is_err());
    }
}
