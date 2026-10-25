//! Existing Document-owned managed Form module edits from a captured SQL view.
//! This closed cohort refuses unexamined owner families. No SQL or file I/O,
//! observer, publisher, metadata rewrite or Form.xml rebuild is performed.
use super::*;
use crate::compiler::{
    bodies::form::{ManagedFormCodecProfile, decode_managed_form},
    families::{
        document::{DocumentMetadataProfile, decode_document_blob},
        form::{FormMetadataProfile, decode_form_metadata_blob},
    },
};
use ibcmd_core::profile::EffectiveProfile;

/// All three codecs are selected together; callers cannot mix their coordinates.
pub struct DocumentFormProfile {
    document: DocumentMetadataProfile,
    metadata: FormMetadataProfile,
    body: ManagedFormCodecProfile,
}
impl DocumentFormProfile {
    pub fn from_effective(profile: &EffectiveProfile) -> Result<Self> {
        Ok(Self {
            document: DocumentMetadataProfile::from_effective(profile)?,
            metadata: FormMetadataProfile::from_effective(profile)?,
            body: ManagedFormCodecProfile::from_effective(profile)?,
        })
    }
}
fn component(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "unsupported SQL Form source name"
    );
    Ok(())
}

/// Resolves root -> Document -> Form BEFORE any UUID/name index reduction.
/// Only Module.bsl is selected; all non-module Form fields must remain exact.
pub fn prepare_form_module(
    snapshot: &Readset,
    selected_path: &str,
    text: &[u8],
    profile: &DocumentFormProfile,
    limits: Limits,
    check: &Check,
) -> Result<ModuleEdit> {
    check()?;
    ensure!(
        text.len() <= limits.source_bytes && !text.contains(&0),
        "SQL Form module source budget or encoding is invalid"
    );
    let expected_text = std::str::from_utf8(text)?.trim_start_matches('\u{feff}');
    let CapturedView {
        rows,
        members,
        configuration,
        old_versions,
        mut budget,
    } = captured_view(snapshot, limits, check)?;
    let mut identities = BTreeSet::from([configuration]);
    for (kind, id) in &members {
        check()?;
        ensure!(
            kind == "Document",
            "SQL Form ownership cohort has an unexamined root family"
        );
        ensure!(
            Uuid::parse_str(id)?.to_string() == *id && identities.insert(id.clone()),
            "duplicate SQL Document membership"
        );
    }
    let route = SourceAssetRegistry
        .route("Form", SourceAssetRole::FormModule)
        .context("no registered Form module role")?;
    let folder =
        crate::mssql_dump::root_family_folder("Document").context("no Document source folder")?;
    let mut parent_paths = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut selected = None;
    for (_, id) in members {
        check()?;
        let blob = rows
            .get(&id)
            .context("SQL Document descriptor is missing")?;
        let descriptor = budget.descriptor(blob)?;
        let owner = decode_document_blob(blob, &profile.document)?;
        ensure!(
            owner.uuid.to_string() == id,
            "SQL Document descriptor identity changed"
        );
        let names = crate::metadata_model::export::object_names("Document", &descriptor)?;
        ensure!(names.uuid == id, "SQL Document naming identity changed");
        let name = names
            .full_name
            .strip_prefix("Document.")
            .context("SQL Document name is not canonical")?;
        component(name)?;
        let parent = format!("{folder}/{name}");
        ensure!(
            parent_paths.insert(parent.to_lowercase()),
            "ambiguous SQL Document source path"
        );
        // Reject collisions with ANY other declared child/type, not just Forms.
        // Each typed descriptor already checks its own original occurrences.
        for child in owner
            .generated_types
            .iter()
            .flat_map(|(a, b)| [a, b])
            .chain(owner.attribute_uuids.iter())
            .chain(owner.command_uuids.iter())
            .chain(owner.form_uuids.iter())
            .chain(owner.template_uuids.iter())
            .chain(owner.addressing_attribute_uuids.iter())
            .chain(owner.dimension_uuids.iter())
            .chain(owner.resource_uuids.iter())
            .chain(owner.recalculation_uuids.iter())
            .chain(owner.content_uuids.iter())
            .chain(owner.child_subsystem_uuids.iter())
            .chain(owner.tabular_sections.iter().flat_map(|section| {
                std::iter::once(&section.uuid).chain(section.attribute_uuids.iter())
            }))
        {
            check()?;
            ensure!(
                identities.insert(child.to_string()),
                "SQL Form has multiple parents or a colliding identity"
            );
        }
        for form_id in owner.form_uuids {
            check()?;
            let form_id = form_id.to_string();
            let metadata = rows
                .get(&form_id)
                .context("SQL Form descriptor is missing")?;
            let _ = budget.descriptor(metadata)?;
            let form = decode_form_metadata_blob(metadata, &profile.metadata)?;
            ensure!(
                form.uuid.to_string() == form_id,
                "SQL Form descriptor identity changed"
            );
            component(&form.name)?;
            let path = format!(
                "{parent}/Forms/{}/{relative}",
                form.name,
                relative = route.relative_path()
            );
            ensure!(
                paths.insert(path.to_lowercase()),
                "ambiguous SQL Form source path"
            );
            if path == selected_path {
                selected = Some(format!("{form_id}{}", route.suffix()));
            }
        }
    }
    let key = selected.context("selection is not one existing Document-owned Form module")?;
    let base = rows
        .get(&key)
        .context("selected SQL Form body is missing")?;
    let plain = budget.inflate(base)?;
    require_nesting_bound(&plain, check)?;
    let old = decode_managed_form(&profile.body, base)?;
    let mut replacements = Rows::new();
    if old.module_text() != expected_text {
        check()?;
        let packed = crate::module_blob::pack_form_body_blob_from_module_text(base, text)?;
        let plain = budget.inflate(&packed.blob)?;
        require_nesting_bound(&plain, check)?;
        let new = decode_managed_form(&profile.body, &packed.blob)?;
        let a = old.parsed();
        let b = new.parsed();
        ensure!(
            b.module_text == expected_text
                && a.revision == b.revision
                && a.layout == b.layout
                && a.trailing == b.trailing
                && a.trailing_fields == b.trailing_fields,
            "SQL Form module overlay changed unselected Form state"
        );
        let versions = rows.get("versions").expect("captured inventory");
        let patched = patch_versions_blob_bytes(versions, std::slice::from_ref(&key), false)?;
        let new_versions = budget.inflate(&patched.blob)?;
        validate_sql_versions_change(&old_versions, &new_versions, &BTreeSet::from([key.clone()]))?;
        replacements.insert(key, packed.blob);
        replacements.insert("versions".into(), patched.blob);
    }
    let digest = snapshot.digest()?;
    check()?;
    Ok(ModuleEdit {
        readset_digest: digest,
        replacements,
    })
}

#[cfg(test)]
#[path = "form_tests.rs"]
mod tests;
