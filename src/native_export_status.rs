//! Native ConfigDumpInfo version status, independent of editor byte comparison.
use crate::mssql_dump::incremental::DumpInfo;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigVersionEntry {
    pub name: String,
    pub id: String,
    pub config_version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDumpInfoSnapshot {
    pub entries: Vec<ConfigVersionEntry>,
    pub parts: BTreeMap<String, String>,
}
impl ConfigDumpInfoSnapshot {
    pub fn parse(xml: &[u8]) -> Result<Self> {
        let parsed = DumpInfo::parse(xml)?;
        Ok(Self {
            entries: parsed
                .entries
                .into_iter()
                .map(|entry| ConfigVersionEntry {
                    name: entry.name,
                    id: entry.id,
                    config_version: entry.config_version,
                })
                .collect(),
            parts: parsed.parts,
        })
    }
    /// Native status ignores a reference-only XML dialect header difference.
    /// An ID removal or rename requires a full export; version changes can
    /// occur even when the exported XML bytes are identical.
    pub fn compare(&self, reference: &Self) -> NativeExportStatus {
        let previous = reference
            .entries
            .iter()
            .map(|entry| (&entry.id, entry))
            .collect::<BTreeMap<_, _>>();
        let now = self
            .entries
            .iter()
            .map(|entry| (&entry.id, entry))
            .collect::<BTreeMap<_, _>>();
        let full_required = reference.entries.iter().any(|entry| {
            now.get(&entry.id)
                .is_none_or(|current| current.name != entry.name)
        }) || reference
            .parts
            .iter()
            .any(|(id, name)| self.parts.get(id) != Some(name));
        let mut changes = self
            .entries
            .iter()
            .filter_map(|entry| {
                let old = previous.get(&entry.id);
                let kind = match old {
                    None => ChangeKind::Added,
                    Some(old)
                        if old.name != entry.name || old.config_version != entry.config_version =>
                    {
                        ChangeKind::Modified
                    }
                    _ => return None,
                };
                Some(VersionChange {
                    name: entry.name.clone(),
                    id: entry.id.clone(),
                    kind,
                    config_version: entry.config_version.clone(),
                    reference_version: old.map(|entry| entry.config_version.clone()),
                })
            })
            .collect::<Vec<_>>();
        changes.sort_by(|a, b| a.name.cmp(&b.name));
        NativeExportStatus {
            full_required,
            changes,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Added,
    Modified,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionChange {
    pub name: String,
    pub id: String,
    pub kind: ChangeKind,
    pub config_version: String,
    pub reference_version: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeExportStatus {
    pub full_required: bool,
    pub changes: Vec<VersionChange>,
}
impl NativeExportStatus {
    pub fn text(&self, short: bool) -> String {
        if self.full_required {
            return format!("{}: all\n", if short { "M" } else { "modified" });
        }
        self.changes
            .iter()
            .map(|change| {
                format!(
                    "{}: {}\n",
                    match (change.kind, short) {
                        (ChangeKind::Added, true) => "A",
                        (ChangeKind::Added, false) => "added",
                        (ChangeKind::Modified, true) => "M",
                        (ChangeKind::Modified, false) => "modified",
                    },
                    change.name
                )
            })
            .collect()
    }
}
