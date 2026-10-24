//! Backend-neutral compilation of selected changes over immutable effective rows.
//!
//! Call under the existing export-engine lock. This planner owns no database
//! session, snapshot, target identity or publication authority. Adapters retain
//! those guards and revalidate the returned source after packing their rows.
use crate::{
    metadata_model::{
        index::{ConfigIndex, ROOT_COLLECTIONS},
        xml::{Element, MetadataXml},
    },
    mssql_source_change::HeldSourceRoot,
    platform::PlatformSpec,
};
use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

// Owns only a private directory created by this request. No Plan needs the
// compiled scratch once its verified rows have been materialized.
pub(crate) struct ImportScratch {
    pub(crate) path: PathBuf,
    identity: fs::Metadata,
}
impl ImportScratch {
    pub(crate) fn create() -> Result<Self> {
        let path =
            std::env::temp_dir().join(format!("ibcmd-editor-import-{}", uuid::Uuid::new_v4()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path)?;
        Ok(Self {
            identity: fs::symlink_metadata(&path)?,
            path,
        })
    }
    pub(crate) fn join(&self, path: &str) -> PathBuf {
        self.path.join(path)
    }
}
impl Drop for ImportScratch {
    fn drop(&mut self) {
        if std::env::var_os("IBCMD_TEST_KEEP_FIXTURES").as_deref()
            == Some(std::ffi::OsStr::new("1"))
        {
            return;
        }
        let _ = &self.identity;
        let Ok(now) = fs::symlink_metadata(&self.path) else {
            return;
        };
        if !now.is_dir() || now.file_type().is_symlink() {
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if (now.dev(), now.ino()) != (self.identity.dev(), self.identity.ino()) {
                return;
            }
        }
        // Runtime-owned temporary data only. remove_dir_all does not follow
        // child symlinks. Lab oracle evidence has a separate, retained location.
        let _ = fs::remove_dir_all(&self.path);
    }
}
#[derive(Default, Clone, Debug)]
pub struct Selection {
    pub files: Vec<String>,
    pub objects: Vec<String>,
    pub changed: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingReference {
    pub path: String,
    pub reference: String,
    pub suggested_object: Option<String>,
}
/// Fully verified packed entries, or reference diagnostics requiring a new plan.
/// Row materialization and target publication remain adapter responsibilities.
pub(crate) struct Prepared {
    pub(crate) source: HeldSourceRoot,
    pub(crate) files: Vec<String>,
    pub(crate) missing_references: Vec<MissingReference>,
    pub(crate) rows: Option<BTreeMap<String, Vec<u8>>>,
}

pub(crate) fn validate_selection(selection: &Selection) -> Result<()> {
    let modes = usize::from(!selection.files.is_empty())
        + usize::from(!selection.objects.is_empty())
        + usize::from(selection.changed);
    if modes != 1 {
        bail!("choose exactly one import selection: files, objects or changed");
    }
    Ok(())
}

pub(crate) fn prepare(
    effective_rows: BTreeMap<String, Vec<u8>>,
    source: HeldSourceRoot,
    selection: &Selection,
    platform: PlatformSpec,
) -> Result<Prepared> {
    validate_selection(selection)?;
    if effective_rows.is_empty() {
        bail!("partial import requires an existing configuration");
    }
    let scratch = ImportScratch::create()?;
    let base = scratch.join("base.cf");
    let tree = scratch.join("expected");
    crate::mssql_dump::config_save::write_rows_cf(effective_rows, &base, false)?;
    let version = platform.xml_version();
    crate::platform::note_export_platform(platform);
    crate::load::export_base(&base, &tree, version)?;
    let baseline = crate::load::relative_files(&tree)?;
    let source_paths = source
        .baseline()
        .files()
        .map(|file| file.path().to_string())
        .collect::<BTreeSet<_>>();
    // A native export's inventory is the synchronization baseline. A
    // difference against a newer target is a conflict, not a local edit.
    let sync = if selection.changed {
        let local = source.source_bytes("ConfigDumpInfo.xml").context("changed import requires a synchronized ConfigDumpInfo.xml; refresh the export baseline or select files explicitly")?;
        Some((
            crate::mssql_dump::incremental::DumpInfo::parse(&local)?,
            crate::mssql_dump::incremental::DumpInfo::read(&tree.join("ConfigDumpInfo.xml"))?,
        ))
    } else {
        None
    };
    let sync_indices = sync.as_ref().map(|(local, current)| {
        (
            crate::mssql_dump::incremental::Ownership::new(&local.entries),
            crate::mssql_dump::incremental::Ownership::new(&current.entries),
            local
                .entries
                .iter()
                .map(|entry| (entry.id.as_str(), entry))
                .collect::<BTreeMap<_, _>>(),
            current
                .entries
                .iter()
                .map(|entry| (entry.id.as_str(), entry))
                .collect::<BTreeMap<_, _>>(),
        )
    });
    let selected = if !selection.files.is_empty() {
        crate::mssql::files_stage::select(source.canonical_root(), &selection.files)?
            .files()
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
    } else {
        let mut prefixes = Vec::new();
        for object in &selection.objects {
            if object == "Configuration" {
                bail!("partial import of Configuration is not supported");
            }
            let owned = crate::stored_objects::source_prefixes(object)
                .ok_or_else(|| anyhow!("invalid object name {object}"))?;
            for prefix in &owned {
                if !baseline.contains(&format!("{prefix}.xml")) {
                    bail!("new object {object} is not supported by partial import");
                }
            }
            prefixes.extend(owned);
        }
        source_paths
            .union(&baseline)
            .filter(|path| {
                if selection.changed {
                    path.as_str() != "ConfigDumpInfo.xml"
                        && crate::stored_objects::object_of_path(path).is_some()
                } else {
                    prefixes.iter().any(|prefix| in_prefix(path, prefix))
                }
            })
            .cloned()
            .collect()
    };
    let mut changed = Vec::new();
    let mut selected_bytes = Vec::new();
    for path in selected {
        crate::cancel::check()?;
        if !baseline.contains(&path) {
            bail!("new file {path} is not supported by partial import; nothing written");
        }
        let bytes = source.source_bytes(&path).with_context(|| {
            format!("missing or changed selected file {path}; deletions are not supported")
        })?;
        let expected = tree.join(&path);
        if *bytes != fs::read(&expected)? {
            if let Some((old_owner, now_owner, old_entries, now_entries)) = &sync_indices {
                let old_id = old_owner.exact(&path).or_else(|| old_owner.folder(&path));
                let now_id = now_owner.exact(&path).or_else(|| now_owner.folder(&path));
                let unchanged = old_id.zip(now_id).is_some_and(|(a, b)| {
                    a == b
                        && old_entries
                            .get(a)
                            .zip(now_entries.get(b))
                            .is_some_and(|(old, now)| {
                                old.name == now.name && old.config_version == now.config_version
                            })
                });
                if !unchanged {
                    bail!(
                        "{path}: target changed since the synchronized export; refresh the baseline or select files explicitly; nothing written"
                    );
                }
            }
            if path == "Configuration.xml" {
                bail!("changed Configuration.xml requires whole-tree import");
            }
            fs::write(&expected, bytes.as_slice())?;
            changed.push(path.clone());
            selected_bytes.push((path, bytes));
        }
    }
    if changed.is_empty() {
        bail!("selected files have no changes");
    }
    let expected_index = ConfigIndex::build(&tree)?;
    let source_xml = source_paths
        .iter()
        .filter(|path| path.ends_with(".xml"))
        .map(|path| {
            Ok((
                source.canonical_root().join(path),
                source.source_bytes(path)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let source_index = ConfigIndex::build_from_files(source.canonical_root(), &source_xml)?;
    let mut missing_references = Vec::new();
    for (path, bytes) in &selected_bytes {
        if path.ends_with(".xml") {
            let doc = MetadataXml::parse(bytes)?;
            references(
                &doc.root,
                path,
                &expected_index,
                &source_index,
                &mut missing_references,
            );
        }
    }
    let ready = missing_references.is_empty();
    let rows = if ready {
        let output = scratch.join("staged.cf");
        crate::load::load_onto_base(&tree, &base, &output, version)
            .map_err(|error| anyhow!("{error}"))?;
        let mut entries = crate::file_infobase::read_cf_entries(&output)?;
        entries.remove("deleted");
        let staged = entries
            .iter()
            .map(|(file_name, bytes)| crate::mssql_dump::StagedRow { file_name, bytes })
            .collect::<Vec<_>>();
        // Always verify; diagnostic environment switches cannot disable editor safety.
        crate::mssql::verify_staged_state(
            &crate::mssql::GuardRequest {
                kind: crate::mssql::StageKind::Patch,
                source_root: &tree,
                tree: crate::mssql::TreeFiles::Unlisted,
                removed: &[],
                path_prefix: &[],
                source_version: Some(version),
            },
            crate::mssql_dump::StateBase::Nothing,
            &staged,
        )?;
        Some(entries)
    } else {
        None
    };
    Ok(Prepared {
        source,
        files: changed,
        missing_references,
        rows,
    })
}

fn in_prefix(path: &str, prefix: &str) -> bool {
    path == prefix
        || path == format!("{prefix}.xml")
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}
pub(crate) fn references(
    node: &Element,
    path: &str,
    target: &ConfigIndex,
    source: &ConfigIndex,
    missing: &mut Vec<MissingReference>,
) {
    if matches!(
        node.name.as_str(),
        "Comment"
            | "Synonym"
            | "Title"
            | "ToolTip"
            | "Description"
            | "Name"
            | "Presentation"
            | "Format"
    ) {
        return;
    }
    let value = node.text.trim();
    let name = value.rsplit(':').next().unwrap_or(value);
    let head = name.split('.').next().unwrap_or_default();
    let is_object = ROOT_COLLECTIONS.iter().any(|(_, kind)| *kind == head);
    let is_type = node.name == "Type"
        && (value.starts_with("cfg:") || (name.contains('.') && head.ends_with("Ref")));
    if (is_object || is_type) && name.contains('.') {
        let standard = name
            .split_once(".StandardAttribute.")
            .is_some_and(|(owner, attribute)| {
                target.objects.contains_key(owner)
                    && crate::metadata_model::objects::standard_markers(head)
                        .is_some_and(|markers| markers.iter().any(|(field, _)| *field == attribute))
            });
        let known = target.objects.contains_key(name)
            || target.children.contains_key(name)
            || target.generated_types.contains_key(name)
            || standard;
        if !known {
            let suggested = if source.objects.contains_key(name) {
                Some(name.to_string())
            } else {
                let short = name.split_once('.').map(|(_, name)| name).unwrap_or(name);
                let candidates = source
                    .objects
                    .values()
                    .filter(|object| object.name == short && head.starts_with(&object.kind))
                    .collect::<Vec<_>>();
                (candidates.len() == 1).then(|| candidates[0].full_name.clone())
            };
            if !missing
                .iter()
                .any(|item| item.path == path && item.reference == value)
            {
                missing.push(MissingReference {
                    path: path.to_string(),
                    reference: value.to_string(),
                    suggested_object: suggested,
                });
            }
        }
    }
    for child in &node.children {
        references(child, path, target, source, missing);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Read, path::Path};

    fn baseline() -> (
        ImportScratch,
        BTreeMap<String, Vec<u8>>,
        PathBuf,
        PlatformSpec,
    ) {
        let scratch = ImportScratch::create().unwrap();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/file-infobase");
        let database = scratch.join("baseline.1CD");
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(fs::File::open(fixture.join("tiny.1CD.gz")).unwrap())
            .read_to_end(&mut bytes)
            .unwrap();
        fs::write(&database, bytes).unwrap();
        let rows = {
            let mut reader = ibcmd_filedb::FileDb::open(&database).unwrap();
            crate::mssql_dump::file_infobase_effective(
                reader.config_rows("CONFIG").unwrap(),
                Vec::new(),
            )
            .unwrap()
            .published
        };
        let cf = scratch.join("baseline.cf");
        crate::mssql_dump::config_save::write_rows_cf(rows.clone(), &cf, false).unwrap();
        let root = scratch.join("source");
        let platform = crate::platform::parse_flag("8.5.1").unwrap();
        crate::load::export_base(&cf, &root, platform.xml_version()).unwrap();
        (scratch, rows, root, platform)
    }
    fn selected(path: &str) -> Selection {
        Selection {
            files: vec![path.into()],
            ..Selection::default()
        }
    }
    fn held(root: &Path) -> HeldSourceRoot {
        HeldSourceRoot::open(
            root,
            crate::mssql_source_change::SourceInventoryLimits::default(),
        )
        .unwrap()
    }

    #[test]
    fn verified_existing_unselected_payload_survives_next_selected_plan() {
        crate::stored_objects::exclusive(|| -> Result<()> {
            let (_scratch, initial, root, platform) = baseline();
            let language = "Languages/English.xml";
            let index = ConfigIndex::build(&root).unwrap();
            let key = index.objects.get("Language.English").unwrap().uuid.clone();
            let text = fs::read_to_string(root.join(language)).unwrap();
            assert!(text.contains("<Comment/>"));
            fs::write(
                root.join(language),
                text.replace("<Comment/>", "<Comment>previous staged edit</Comment>"),
            )
            .unwrap();
            let first = prepare(initial, held(&root), &selected(language), platform).unwrap();
            assert!(first.missing_references.is_empty());
            let previous = first.rows.unwrap();
            let previous_language = previous.get(&key).unwrap().clone();
            let module = "CommonModules/ФайловаяБазаПроверка/Ext/Module.bsl";
            let mut body = fs::read(root.join(module)).unwrap();
            body.extend_from_slice(b"\r\n// next selected edit\r\n");
            fs::write(root.join(module), body).unwrap();
            let second =
                prepare(previous.clone(), held(&root), &selected(module), platform).unwrap();
            assert_eq!(second.files, vec![module.to_string()]);
            assert!(second.missing_references.is_empty());
            assert_eq!(
                second.rows.as_ref().unwrap().get(&key).unwrap(),
                &previous_language
            );
            assert_eq!(previous.get(&key).unwrap(), &previous_language);
            second.source.require_unchanged().unwrap();
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn missing_existing_and_new_files_are_refused() {
        crate::stored_objects::exclusive(|| -> Result<()> {
            let (scratch, rows, root, platform) = baseline();
            let existing = "CommonModules/ФайловаяБазаПроверка/Ext/Module.bsl";
            assert!(root.join(existing).is_file());
            fs::rename(
                root.join(existing),
                scratch.join("retained-original-module.bsl"),
            )
            .unwrap();
            assert!(!root.join(existing).exists());
            assert!(prepare(rows.clone(), held(&root), &selected(existing), platform).is_err());
            let new = "CommonModules/ФайловаяБазаПроверка/Ext/Additional.bsl";
            fs::write(root.join(new), b"// unsupported new body").unwrap();
            let error = prepare(rows, held(&root), &selected(new), platform)
                .err()
                .unwrap();
            assert!(error.to_string().contains("new file"), "{error:#}");
            Ok(())
        })
        .unwrap();
    }
}
