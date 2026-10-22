//! Narrow row-identity adapter; native comparison remains owned by #362.
use super::{DatabaseVersionEvidence, ObjectStatus, StoredObject};
use crate::native_export_status::{ConfigDumpInfoSnapshot, NativeExportStatus};
use std::collections::{BTreeMap, BTreeSet};

fn owner<'a>(id: &str, name: &str, objects: &'a [StoredObject]) -> Option<&'a StoredObject> {
    // Engine rows_under uses this exact UUID / UUID.<row-role> ownership rule.
    // Check canonical name too; no FQN splitting or guessed child/module routes.
    let candidates = objects
        .iter()
        .filter(|object| {
            let same_id = id.eq_ignore_ascii_case(&object.uuid)
                || id
                    .get(..object.uuid.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(&object.uuid))
                    && id
                        .get(object.uuid.len()..)
                        .is_some_and(|suffix| suffix.starts_with('.'));
            let same_name = if object.kind == "Configuration" {
                name == "Configuration.Configuration"
            } else {
                name == object.full_name
                    || object.kind == "CommonModule"
                        && name == format!("{}.Module", object.full_name)
            };
            same_id && same_name
        })
        .collect::<Vec<_>>();
    (candidates.len() == 1).then(|| candidates[0])
}

pub(super) fn attach(
    objects: &[StoredObject],
    current: &ConfigDumpInfoSnapshot,
    native: &NativeExportStatus,
    baseline_identity: &str,
    database_snapshot: &str,
    statuses: &mut [ObjectStatus],
) {
    let changed = native
        .changes
        .iter()
        .map(|c| c.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut flags = BTreeMap::<String, bool>::new();
    let mut complete = !native.full_required;
    for entry in &current.entries {
        if let Some(object) = owner(&entry.id, &entry.name, objects) {
            *flags.entry(object.full_name.clone()).or_default() |=
                changed.contains(entry.id.as_str());
        } else {
            // Unmapped or ambiguous coverage may affect an owner: never infer clean.
            complete = false;
        }
    }
    for row in statuses {
        let flag = flags.get(&row.object).copied();
        let conclusive = complete && flag.is_some() && row.version.is_some();
        row.database_version_status = Some(DatabaseVersionEvidence {
            basis: "native-config-version",
            object: row.object.clone(),
            object_version: row.version.clone(),
            baseline_identity: baseline_identity.into(),
            database_snapshot: database_snapshot.into(),
            conclusive,
            changed: conclusive.then_some(flag.unwrap_or(false)),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn object(name: &str, id: &str, kind: &str) -> StoredObject {
        StoredObject {
            kind: kind.into(),
            name: name.into(),
            full_name: name.into(),
            uuid: id.into(),
            path: String::new(),
            folder: String::new(),
        }
    }
    #[test]
    fn identity_and_canonical_name_must_agree_unambiguously() {
        let objects = vec![
            object("CommonModule.Test", "id", "CommonModule"),
            object("Configuration", "root", "Configuration"),
        ];
        assert_eq!(
            owner("id.0", "CommonModule.Test.Module", &objects)
                .unwrap()
                .full_name,
            "CommonModule.Test"
        );
        assert_eq!(
            owner("root", "Configuration.Configuration", &objects)
                .unwrap()
                .full_name,
            "Configuration"
        );
        assert!(owner("id", "CommonModule.Other", &objects).is_none());
        assert!(owner("id-other", "CommonModule.Test", &objects).is_none());
        let nested = vec![
            object("Catalog.Parent", "parent", "Catalog"),
            object("Catalog.Parent.Form.Child", "child", "Form"),
        ];
        assert!(owner("parent.0", "Catalog.Parent.Form.Child.Form", &nested).is_none());
        assert!(owner("body-row", "Catalog.Parent.Form.Child", &nested).is_none());
        assert!(owner("parent.0", "Catalog.Parent.ObjectModule", &nested).is_none());
        let duplicate = vec![objects[0].clone(), objects[0].clone()];
        assert!(owner("id.0", "CommonModule.Test.Module", &duplicate).is_none());
    }
    #[test]
    fn mssql_refuses_native_status_before_opening_files_or_sql() {
        let mut configuration = super::super::StoredConfiguration::new(
            super::super::ConfigSource::Database {
                sql: crate::sql::SqlExec::detached("must not query"),
                database: "unused".into(),
            },
            crate::platform::parse("8.5.1").unwrap(),
        );
        let error = configuration
            .status_with_versions(
                None,
                std::path::Path::new("absent"),
                std::path::Path::new("absent"),
                &|_, _, _| {},
            )
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("coherent main configuration view")
        );
    }
}
