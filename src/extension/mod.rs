//! Configuration extensions (.cfe). File and database exports use the same
//! adoption-aware source pipeline. The file adapter validates the CFE manifest
//! and retains its own reference context instead of fetching a base database.
//!
//! Every adopted object -- the root included -- carries its adoption in the
//! header after the comment: `…,"comment",1,N,(property uuid,state)×N,
//! <extended object uuid>,0}` (`0,0,<nil>,0` for an own object). A listed
//! property is controlled by the extension: printed as a property element, or
//! as `<xr:PropertyState>` `Extended` for a module or command interface the
//! extension extends.

pub mod adopted;
pub mod adoption;
pub mod predefined;
pub mod root;

pub(crate) mod build;

#[cfg(test)]
use std::fs;
use std::path::Path;

use anyhow::Result;
#[cfg(test)]
use anyhow::{Context, bail};
use ibcmd_cf::archive::PackedCfArchive;
#[cfg(test)]
use ibcmd_cf::export::{StorageExportDisposition, StorageExportEntryReport};

use crate::{
    legacy_version::InfobaseConfigSourceVersion,
    mssql_dump::{self, StorageImageSourceExportReport},
};

#[cfg(test)]
use crate::{external::brace, module_blob::inflate_raw};

/// An extension container keeps `configinfo` and no `root`; a configuration
/// has `root` (and `version`/`versions`), an external object has `root`.
pub fn is_extension(archive: &PackedCfArchive) -> bool {
    let has = |wanted: &str| archive.entries().iter().any(|entry| entry.name() == wanted);
    has("configinfo") && !has("root")
}

/// `mssql_dump::export_packed_cf_archive_to_source`, with an extension's
/// output rewritten as the platform dumps an extension.
pub fn export_packed_cf_archive_to_source(
    archive: PackedCfArchive,
    output_dir: &Path,
    overwrite: bool,
    source_version: InfobaseConfigSourceVersion,
) -> Result<StorageImageSourceExportReport> {
    if !is_extension(&archive) {
        return mssql_dump::export_packed_cf_archive_to_source(
            archive,
            output_dir,
            overwrite,
            source_version,
        );
    }
    let (_, source_profile, entries) = archive.into_parts();
    mssql_dump::extension::export_extension_entries_to_source(
        source_profile.as_str(),
        entries
            .into_iter()
            .map(|entry| entry.into_parts())
            .collect(),
        output_dir,
        overwrite,
        source_version,
    )
}

/// [`export_packed_cf_archive_to_source`] for `(name, packed payload)`
/// entries already taken out of the container -- a subset of them, for an
/// incremental export (`crate::update`).
pub fn export_entries_to_source(
    source_profile: &str,
    entries: Vec<(String, Vec<u8>)>,
    output_dir: &Path,
    overwrite: bool,
    source_version: InfobaseConfigSourceVersion,
) -> Result<StorageImageSourceExportReport> {
    let has = |wanted: &str| entries.iter().any(|(name, _)| name == wanted);
    if has("configinfo") && !has("root") {
        return mssql_dump::extension::export_extension_entries_to_source(
            source_profile,
            entries,
            output_dir,
            overwrite,
            source_version,
        );
    }
    mssql_dump::export_packed_entries_to_source(
        source_profile,
        entries,
        output_dir,
        overwrite,
        source_version,
        None,
    )
}

/// Rewrites the metadata XML of every adopted object (the root aside). An
/// object whose adoption names a property no probe has set apart keeps every
/// property and says so in its report entry.
#[cfg(test)]
fn finish_adopted(
    entries: &[(String, Vec<u8>)],
    output_dir: &Path,
    report: &mut StorageImageSourceExportReport,
) -> Result<()> {
    let rows = Rows::new(entries);
    let root = rows.root().map(|(uuid, _)| uuid).ok();
    let types = std::cell::OnceCell::new();
    // Report entries by key, found once per adopted row.
    let mut report_index = std::collections::HashMap::with_capacity(report.storage.entries.len());
    for (index, entry) in report.storage.entries.iter().enumerate() {
        report_index
            .entry(entry.logical_key.to_ascii_lowercase())
            .or_insert(index);
    }
    for (name, _) in entries {
        if name.len() != 36 || name.contains('.') || Some(name) == root.as_ref() {
            continue;
        }
        let Some(text) = rows.text(name) else {
            continue;
        };
        let Some(adoption) = adoption::header_of(&text, name).and_then(adoption::parse) else {
            continue;
        };
        if !adoption.adopted {
            continue;
        }
        let Some(&index) = report_index.get(&name.to_ascii_lowercase()) else {
            continue;
        };
        let entry = &mut report.storage.entries[index];
        let Some(rel) = entry
            .outputs
            .iter()
            .find(|output| output.ends_with(".xml") && !output.replace('\\', "/").contains("/Ext/"))
            .cloned()
        else {
            continue;
        };
        let path = output_dir.join(&rel);
        // A widened type's checked type, from the element's own type field.
        let checked = |element: &str, indent: &str| -> Result<String> {
            let field = adoption::field_after_header(&text, element)
                .with_context(|| format!("no type field after the header of {element}"))?;
            types
                .get_or_init(|| mssql_dump::extension_types::ExtensionTypes::from_entries(entries))
                .render(field, indent)
                .with_context(|| format!("checked type of {element} `{field}` is not known"))
        };
        let rewritten = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))
            .and_then(|xml| {
                let (xml, mut unknown) = adopted::rewrite(&xml, &adoption, &checked)
                    .with_context(|| format!("adopted object {rel}"))?;
                let child_adoption =
                    |child: &str| adoption::header_of(&text, child).and_then(adoption::parse);
                let (xml, child_unknown) =
                    adopted::rewrite_children(&xml, &child_adoption, &checked)
                        .with_context(|| format!("adopted children of {rel}"))?;
                unknown.0.extend(child_unknown.0);
                unknown.0.sort();
                unknown.0.dedup();
                Ok((xml, unknown))
            })
            .and_then(|(rewritten, unknown)| {
                fs::write(&path, rewritten)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                Ok(unknown)
            })
            .and_then(|unknown| {
                finish_predefined(&rows, name, &path)?;
                Ok(unknown)
            });
        match rewritten {
            Ok(unknown) if !unknown.0.is_empty() => {
                entry.message = Some(format!(
                    "extension writer: controlled properties {} are not known; every property kept",
                    unknown.0.join(", ")
                ));
            }
            Ok(_) => {}
            // The object keeps the pipeline's rendering and is reported, as the
            // root is (`degrade_root`); the rest of the export stands.
            Err(error) => {
                entry.disposition = StorageExportDisposition::Failed;
                entry.message = Some(format!("extension writer: {error:#}"));
            }
        }
    }
    recount(&mut report.storage);
    Ok(())
}

/// The adopted object `uuid`'s `Ext/Predefined.xml` (beside its metadata XML
/// at `path`) with each item's `<ExtensionState>` (see `predefined`).
#[cfg(test)]
fn finish_predefined(rows: &Rows, uuid: &str, path: &Path) -> Result<()> {
    let predefined = path.with_extension("").join("Ext").join("Predefined.xml");
    let Some(text) = rows.text(&format!("{uuid}.1c")) else {
        return Ok(());
    };
    if !predefined.exists() {
        return Ok(());
    }
    let states = predefined::states(&text).context("predefined items")?;
    let xml = fs::read_to_string(&predefined)
        .with_context(|| format!("failed to read {}", predefined.display()))?;
    let rewritten = predefined::rewrite(&xml, &states)?;
    fs::write(&predefined, rewritten)
        .with_context(|| format!("failed to write {}", predefined.display()))
}

#[cfg(test)]
fn recount(storage: &mut ibcmd_cf::export::StorageExportReport) {
    let count = |wanted: StorageExportDisposition| {
        storage
            .entries
            .iter()
            .filter(|entry| entry.disposition == wanted)
            .count()
    };
    storage.failed = count(StorageExportDisposition::Failed);
    storage.supported = count(StorageExportDisposition::Supported);
    storage.opaque = count(StorageExportDisposition::Opaque);
    storage.logical_entries = storage.entries.len();
}
/// The container's rows, decoded on demand and found by name through an
/// index built once (a lookup per row was quadratic on a large extension).
#[cfg(test)]
struct Rows<'a> {
    entries: &'a [(String, Vec<u8>)],
    by_name: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
impl<'a> Rows<'a> {
    fn new(entries: &'a [(String, Vec<u8>)]) -> Self {
        let mut by_name = std::collections::HashMap::with_capacity(entries.len());
        for (index, (name, _)) in entries.iter().enumerate() {
            by_name.entry(name.to_ascii_lowercase()).or_insert(index);
        }
        Self { entries, by_name }
    }

    fn text(&self, name: &str) -> Option<String> {
        let (_, payload) = &self.entries[*self.by_name.get(&name.to_ascii_lowercase())?];
        String::from_utf8(inflate_raw(payload).ok()?).ok()
    }

    /// The configuration row: `{2,{<its uuid>},N,{9cd510cd-…,{1,{<tuple>…`,
    /// the tuple `{68,…}` (8.5: `{76,…}`, older editions `{59..67,…}`).
    fn root(&self) -> Result<(String, String)> {
        for (name, _) in self.entries {
            if name.len() != 36 || name.contains('.') {
                continue;
            }
            let Some(text) = self.text(name) else {
                continue;
            };
            let body = brace::strip_bom(&text).trim_start();
            // The head is cut in bytes: byte 200 may fall inside a letter.
            let head = &body.as_bytes()[..body.len().min(200)];
            let own = format!("{{{name}}}");
            if body.starts_with("{2,")
                && body.contains("{9cd510cd-abfc-11d4-9434-004095e12fc7,")
                && head
                    .windows(own.len())
                    .any(|window| window == own.as_bytes())
            {
                return Ok((name.clone(), body.to_owned()));
            }
        }
        bail!("no configuration row `{{2,{{uuid}},…,{{9cd510cd-…,{{1,{{…}}}}}}…`")
    }

    /// The name in the header of the row `uuid`.
    fn name(&self, uuid: &str) -> Option<String> {
        let text = self.text(uuid)?;
        let marker = format!("{{1,0,{uuid}}},");
        let at = text.find(&marker)? + marker.len();
        root::quoted(text[at..].trim_start()).map(|(value, _)| value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ibcmd_cf::export::StorageExportReport;

    const GOOD: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    const BAD: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";

    fn adopted_row(uuid: &str) -> Vec<u8> {
        let text = format!(
            "{{1,{{0,{{3,{{1,0,{uuid}}},\"Имя\",{{0}},\"\",1,0,\
             00000000-0000-0000-0000-000000000001,0}}}},0}}"
        );
        crate::module_blob::deflate_raw(text.as_bytes()).unwrap()
    }

    fn entry(uuid: &str, output: &str) -> StorageExportEntryReport {
        StorageExportEntryReport::supported_packed(uuid, uuid, 1, 1, vec![output.to_owned()])
    }

    #[test]
    fn a_row_whose_200th_byte_splits_a_letter_is_read_without_a_panic() {
        // A long Cyrillic name puts byte 200 inside a two-byte letter; the
        // root search only looks at the head of each row.
        // Not the root: its header names another uuid.
        let mut head = format!(
            "{{2,{{cccccccc-cccc-4ccc-8ccc-cccccccccccc}},1,{{9cd510cd-abfc-11d4-9434-004095e12fc7,\""
        );
        if (200 - head.len()) % 2 == 0 {
            head.push('x');
        }
        let other = format!("{head}{}\"}}}}", "Я".repeat(200));
        assert!(
            !other.is_char_boundary(200),
            "the fixture must split a letter at 200"
        );
        let root =
            format!("{{2,{{{GOOD}}},1,{{9cd510cd-abfc-11d4-9434-004095e12fc7,\"Корень\"}}}}");
        let entries = vec![
            (
                BAD.to_owned(),
                crate::module_blob::deflate_raw(other.as_bytes()).unwrap(),
            ),
            (
                GOOD.to_owned(),
                crate::module_blob::deflate_raw(root.as_bytes()).unwrap(),
            ),
        ];
        let rows = Rows::new(&entries);
        let (uuid, _) = rows.root().unwrap();
        assert_eq!(uuid, GOOD);
    }

    #[test]
    fn an_adopted_object_the_writer_cannot_read_fails_alone() {
        let dir =
            std::env::temp_dir().join(format!("ibcmd-adopted-degrade-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Catalogs")).unwrap();
        let good = "<MetaDataObject>\r\n\t<Catalog uuid=\"x\">\r\n\t\t<Properties>\r\n\t\t\t<Name>Имя</Name>\r\n\t\t\t<Comment/>\r\n\t\t</Properties>\r\n\t</Catalog>\r\n</MetaDataObject>";
        fs::write(dir.join("Catalogs/Хорошо.xml"), good).unwrap();
        fs::write(dir.join("Catalogs/Плохо.xml"), "<MetaDataObject/>").unwrap();
        let entries = vec![
            (GOOD.to_owned(), adopted_row(GOOD)),
            (BAD.to_owned(), adopted_row(BAD)),
        ];
        let mut report = StorageImageSourceExportReport {
            output_dir: dir.clone(),
            source_version: "2.20".to_owned(),
            files_written: 2,
            storage: StorageExportReport::from_parts(
                None,
                2,
                2,
                vec![
                    entry(GOOD, "Catalogs/Хорошо.xml"),
                    entry(BAD, "Catalogs/Плохо.xml"),
                ],
            ),
        };

        finish_adopted(&entries, &dir, &mut report).unwrap();

        let bad = &report.storage.entries[1];
        assert_eq!(bad.disposition, StorageExportDisposition::Failed);
        assert!(
            bad.message
                .as_deref()
                .unwrap_or_default()
                .contains("no <Properties>")
        );
        assert_eq!(report.storage.failed, 1);
        assert_eq!(
            report.storage.entries[0].disposition,
            StorageExportDisposition::Supported
        );
        let rewritten = fs::read_to_string(dir.join("Catalogs/Хорошо.xml")).unwrap();
        assert!(
            rewritten.contains("<ObjectBelonging>Adopted</ObjectBelonging>"),
            "{rewritten}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
