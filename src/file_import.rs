//! Safe editor partial CONFIGSAVE staging on real file infobases (#382).
//! Preflight builds and verifies a complete effective tree; publication uses
//! an opaque plan bound to the target identity and original source census.
use crate::{
    metadata_model::{
        index::{ConfigIndex, ROOT_COLLECTIONS},
        xml::{Element, MetadataXml},
    },
    mssql_source_change::{HeldSourceRoot, SourceInventoryLimits},
    platform::PlatformSpec,
};
use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

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
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preflight {
    pub target: PathBuf,
    pub files: Vec<String>,
    pub missing_references: Vec<MissingReference>,
    pub ready: bool,
}
/// Contents are private: callers cannot substitute compiled rows or fingerprints.
pub struct Plan {
    summary: Preflight,
    snapshot: String,
    source: HeldSourceRoot,
    rows: Option<Vec<ibcmd_filedb::ConfigRow>>,
}
impl Plan {
    pub fn summary(&self) -> &Preflight {
        &self.summary
    }

    /// The caller supplies its CURRENT connection target after user confirmation.
    /// A changed target, source or CONFIG/CONFIGSAVE invalidates the entire plan.
    pub fn publish(self, current_target: &Path) -> Result<ibcmd_filedb::ReplaceReport> {
        self.publish_guarded(current_target, &mut |_| Ok(()))
    }

    /// Operator policy can bind the actual consumed descriptor to its pinned identity.
    pub fn publish_guarded(
        self,
        current_target: &Path,
        guard: &mut dyn FnMut(&fs::Metadata) -> Result<()>,
    ) -> Result<ibcmd_filedb::ReplaceReport> {
        crate::stored_objects::exclusive(|| {
            crate::cancel::check()?;
            let current = fs::canonicalize(ibcmd_filedb::data_file(current_target))?;
            if current != self.summary.target {
                bail!("connection changed since import preflight; nothing written");
            }
            let rows = self.rows.as_ref().ok_or_else(|| anyhow!("unresolved references; confirm dependencies and repeat preflight before importing"))?;
            self.source.require_unchanged()?;
            let session = ibcmd_filedb::open_for_write(&current)?;
            guard(&session.metadata()?)?;
            if session.snapshot()? != self.snapshot {
                bail!("file database changed since import preflight; nothing written");
            }
            let result = session.replace_guarded("CONFIGSAVE", rows, |metadata| {
                self.source
                    .require_unchanged()
                    .map_err(|error| ibcmd_filedb::Error::Refused(error.to_string()))?;
                guard(metadata)
                    .map_err(|error| ibcmd_filedb::Error::Refused(format!("{error:#}")))?;
                // The source census/policy can take time. Cancellation must be
                // the final action before the format layer publishes its copy.
                crate::cancel::check()
                    .map_err(|error| ibcmd_filedb::Error::Refused(error.to_string()))
            });
            // Preserve the typed cancellation for the RPC layer.
            if result.is_err() {
                crate::cancel::check()?;
            }
            result.map_err(Into::into)
        })
    }
}

pub fn prepare(
    target: &Path,
    source: &Path,
    selection: &Selection,
    platform: PlatformSpec,
) -> Result<Plan> {
    prepare_guarded(target, source, selection, platform, &mut |_| Ok(()))
}

pub fn prepare_guarded(
    target: &Path,
    source: &Path,
    selection: &Selection,
    platform: PlatformSpec,
    guard: &mut dyn FnMut(&fs::Metadata) -> Result<()>,
) -> Result<Plan> {
    crate::stored_objects::exclusive(|| prepare_locked(target, source, selection, platform, guard))
}
fn in_prefix(path: &str, prefix: &str) -> bool {
    path == prefix
        || path == format!("{prefix}.xml")
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}
fn prepare_locked(
    target: &Path,
    source: &Path,
    selection: &Selection,
    platform: PlatformSpec,
    guard: &mut dyn FnMut(&fs::Metadata) -> Result<()>,
) -> Result<Plan> {
    crate::cancel::check()?;
    let modes = usize::from(!selection.files.is_empty())
        + usize::from(!selection.objects.is_empty())
        + usize::from(selection.changed);
    if modes != 1 {
        bail!("choose exactly one import selection: files, objects or changed");
    }
    let source = HeldSourceRoot::open(source, SourceInventoryLimits::default())?;
    let target = fs::canonicalize(ibcmd_filedb::data_file(target)).context("file import target")?;
    let mut session = ibcmd_filedb::open_for_write(&target)?;
    guard(&session.metadata()?)?;
    let snapshot = session.snapshot()?;
    let config = session.config_rows("CONFIG")?;
    let saved = session.config_rows("CONFIGSAVE")?;
    let effective = crate::mssql_dump::file_infobase_effective(config, saved)?;
    if effective.published.is_empty() {
        bail!("partial import requires an existing configuration");
    }
    let scratch =
        std::env::temp_dir().join(format!("ibcmd-editor-import-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&scratch)?;
    let base = scratch.join("base.cf");
    let tree = scratch.join("expected");
    crate::mssql_dump::config_save::write_rows_cf(effective.published, &base, false)?;
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
        let deleted = effective
            .stored_names
            .iter()
            .filter(|name| !entries.contains_key(*name))
            .cloned()
            .collect::<Vec<_>>();
        Some(crate::file_infobase::stage_rows(&entries, &deleted)?)
    } else {
        None
    };
    source.require_unchanged()?;
    crate::cancel::check()?;
    guard(&session.metadata()?)?;
    if session.snapshot()? != snapshot {
        bail!("file database changed during preflight; nothing written");
    }
    crate::cancel::check()?;
    Ok(Plan {
        summary: Preflight {
            target,
            files: changed,
            missing_references,
            ready,
        },
        snapshot,
        source,
        rows,
    })
}

fn references(
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
    #[test]
    fn reference_looking_plain_text_is_not_a_dependency() {
        let doc=MetadataXml::parse(br#"<MetaDataObject><Catalog><Properties><Comment>Catalog.DoesNotExist</Comment><Synonym><item><content>Document.AlsoMissing</content></item></Synonym></Properties></Catalog></MetaDataObject>"#).unwrap();
        let mut missing = Vec::new();
        references(
            &doc.root,
            "Catalogs/Example.xml",
            &ConfigIndex::default(),
            &ConfigIndex::default(),
            &mut missing,
        );
        assert!(missing.is_empty());
    }
    #[test]
    fn missing_metadata_type_suggests_the_source_object() {
        let mut source = ConfigIndex::default();
        source.objects.insert(
            "Catalog.Dependency".into(),
            crate::metadata_model::index::ObjectEntry {
                kind: "Catalog".into(),
                name: "Dependency".into(),
                uuid: uuid::Uuid::new_v4().to_string(),
                full_name: "Catalog.Dependency".into(),
                path: "Catalogs/Dependency.xml".into(),
            },
        );
        let doc=MetadataXml::parse(br#"<MetaDataObject><Attribute><Type><Type>cfg:CatalogRef.Dependency</Type></Type></Attribute></MetaDataObject>"#).unwrap();
        let mut missing = Vec::new();
        references(
            &doc.root,
            "Documents/Example.xml",
            &ConfigIndex::default(),
            &source,
            &mut missing,
        );
        assert_eq!(missing.len(), 1);
        assert_eq!(
            missing[0].suggested_object.as_deref(),
            Some("Catalog.Dependency")
        );
    }
}
