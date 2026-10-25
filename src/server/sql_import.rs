//! Bounded source admission for SQL editor preparation.
//!
//! CommonModule preparation binds original source custody, observed storage
//! semantics and a full physical snapshot to one consumed server plan. Publication
//! stages text and versions only; native activation remains a separate operation.
pub mod compiler_diagnostic;
pub mod coverage;
pub mod readset;
pub mod selected;
pub mod snapshot;
pub mod transaction;

use std::{collections::BTreeMap, path::Path, sync::Arc};

use anyhow::{Context, Result, bail};

use crate::{
    file_import::Selection,
    mssql_source_change::{HeldSourceRoot, SourceInventoryLimits, SourceTraversalLimits},
};

/// Operator-side bounds, never supplied by an editor request.
#[derive(Clone, Copy)]
pub struct SourceLimits {
    pub inventory: SourceInventoryLimits,
    pub traversal: SourceTraversalLimits,
}
impl Default for SourceLimits {
    fn default() -> Self {
        Self {
            inventory: SourceInventoryLimits {
                max_files: 10_000,
                max_file_bytes: 16 * 1024 * 1024,
                max_total_bytes: 128 * 1024 * 1024,
            },
            traversal: SourceTraversalLimits {
                max_entries: 20_000,
                max_depth: 64,
            },
        }
    }
}

/// Original-census bytes only. This is source admission, not SQL write authority.
pub struct SourceAdmission {
    source: HeldSourceRoot,
    selected: BTreeMap<String, Arc<Vec<u8>>>,
}
impl SourceAdmission {
    pub fn prepare(
        root: &Path,
        selection: &Selection,
        verify: Option<bool>,
        limits: SourceLimits,
    ) -> Result<Self> {
        let modes = usize::from(!selection.files.is_empty())
            + usize::from(!selection.objects.is_empty())
            + usize::from(selection.changed);
        if modes != 1 {
            bail!("choose exactly one SQL import selection: files, objects or changed");
        }
        if verify == Some(false) {
            bail!("SQL editor import requires mandatory verification");
        }
        for object in &selection.objects {
            let prefixes = crate::stored_objects::source_prefixes(object)
                .with_context(|| format!("invalid SQL import object {object}"))?;
            if prefixes
                .iter()
                .any(|prefix| prefix == "Configuration" || prefix == "Ext")
            {
                bail!("partial SQL import of Configuration is unsupported");
            }
        }
        let source =
            HeldSourceRoot::open_with_traversal_limits(root, limits.inventory, limits.traversal)?;
        let mut selected = BTreeMap::new();
        for file in &selection.files {
            if file == "Configuration.xml" {
                bail!("Configuration.xml requires a whole-tree import");
            }
            // The held census checks exact spelling, aliases and original file
            // identity before returning bytes. A filesystem selector cannot
            // authorize a replacement that appeared after the census.
            selected.insert(file.clone(), source.source_bytes(file)?);
        }
        if !selection.files.is_empty() {
            crate::mssql::files_stage::select(source.canonical_root(), &selection.files)?;
        }
        if selection.changed {
            let inventory = source
                .source_bytes("ConfigDumpInfo.xml")
                .context("SQL changed import requires synchronized ConfigDumpInfo.xml")?;
            crate::mssql_dump::incremental::DumpInfo::parse(&inventory)?;
            selected.insert("ConfigDumpInfo.xml".into(), inventory);
        }
        source.require_unchanged()?;
        Ok(Self { source, selected })
    }

    /// Compile exactly one admitted CommonModule file from retained bytes.
    /// The result is not publication authority: target/profile, one-shot plan
    /// and full transactional CAS checks are still required by the adapter.
    pub fn prepare_common_module(
        &self,
        snapshot: &readset::Readset,
        selected_path: &str,
        profile: &crate::compiler::families::modules::CommonModuleProfile,
        limits: selected::Limits,
        check: &readset::Check,
    ) -> Result<selected::ModuleEdit> {
        self.prepare_selected_module(selected_path, check, |bytes| {
            selected::prepare_module_file(snapshot, selected_path, bytes, profile, limits, check)
        })
    }

    /// Compile existing CommonModule layout from held bytes, without a claimed
    /// platform coordinate. This still supplies no schema or write admission.
    pub fn prepare_common_module_layout(
        &self,
        snapshot: &readset::Readset,
        selected_path: &str,
        limits: selected::Limits,
        check: &readset::Check,
    ) -> Result<selected::ModuleEdit> {
        self.prepare_selected_module(selected_path, check, |bytes| {
            selected::prepare_module_file_layout(snapshot, selected_path, bytes, limits, check)
        })
    }

    /// Compile one existing Document-owned Form module from retained bytes.
    /// This does not admit Form.xml, other owner families or SQL publication.
    pub fn prepare_document_form_module(
        &self,
        snapshot: &readset::Readset,
        selected_path: &str,
        profile: &selected::form::DocumentFormProfile,
        limits: selected::Limits,
        check: &readset::Check,
    ) -> Result<selected::ModuleEdit> {
        self.prepare_selected_module(selected_path, check, |bytes| {
            selected::form::prepare_form_module_file(
                snapshot,
                selected_path,
                bytes,
                profile,
                limits,
                check,
            )
        })
    }

    /// Compile one held existing Document Form module by its original grammar.
    /// Target/schema admission and native acceptance are independent boundaries.
    pub fn prepare_document_form_module_layout(
        &self,
        snapshot: &readset::Readset,
        selected_path: &str,
        limits: selected::Limits,
        check: &readset::Check,
    ) -> Result<selected::ModuleEdit> {
        self.prepare_selected_module(selected_path, check, |bytes| {
            selected::form::prepare_form_module_file_layout(
                snapshot,
                selected_path,
                bytes,
                limits,
                check,
            )
        })
    }

    fn prepare_selected_module(
        &self,
        selected_path: &str,
        check: &readset::Check,
        compile: impl FnOnce(&[u8]) -> Result<selected::ModuleEdit>,
    ) -> Result<selected::ModuleEdit> {
        check()?;
        if self.selected.len() != 1 {
            bail!("SQL module preparation requires one explicit admitted file");
        }
        let bytes = self
            .bytes(selected_path)
            .context("SQL module file is outside the admitted selection")?;
        self.revalidate()?;
        let edit = compile(bytes)?;
        check()?;
        self.revalidate()?;
        Ok(edit)
    }

    pub fn bytes(&self, path: &str) -> Option<&[u8]> {
        self.selected.get(path).map(|bytes| bytes.as_slice())
    }

    /// Revalidates original bytes, file/ancestor witnesses and the bounded
    /// anchored file/directory census, including empty directories.
    /// This is a revalidation boundary, not an atomic filesystem freeze.
    /// This must precede the guarded SQL publication boundary, never follow it.
    pub fn revalidate(&self) -> Result<()> {
        self.source.require_unchanged().map_err(Into::into)
    }
}

/// Server-owned prepared existing-CommonModule operation. No fields, raw rows,
/// profile assertions or compiled payload can be supplied by an RPC caller.
pub struct CommonModulePlan {
    source: SourceAdmission,
    source_dir: std::path::PathBuf,
    original: readset::Readset,
    edit: selected::ModuleEdit,
    storage: snapshot::CommonModuleStorageProfile,
    policy: transaction::WritePolicy,
    summary: CommonModulePreflight,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonModulePreflight {
    backend: &'static str,
    target: String,
    files: Vec<String>,
    missing_references: Vec<crate::file_import::MissingReference>,
    ready: bool,
    staged_rows: usize,
}
/// Unsupported request shape, distinct from source, target or SQL failures.
#[derive(Debug)]
pub struct RequestRefused(&'static str);
impl std::fmt::Display for RequestRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for RequestRefused {}

impl CommonModulePlan {
    /// Read-only preparation of one explicit canonical existing module path.
    /// There is no platform-build inference, arbitrary multipart policy or
    /// user-supplied schema admission; full snapshot CAS remains mandatory.
    pub fn prepare(
        client: &dyn crate::sql::SqlClient,
        database: &str,
        source_dir: &Path,
        selection: &Selection,
        verify: Option<bool>,
        check: readset::Check,
    ) -> Result<Self> {
        check()?;
        if verify == Some(false) {
            return Err(RequestRefused("SQL editor import requires mandatory verification").into());
        }
        if selection.files.len() != 1 || !selection.objects.is_empty() || selection.changed {
            return Err(RequestRefused(
                "SQL preflight currently supports one existing CommonModule file",
            )
            .into());
        }
        let path = &selection.files[0];
        let route = crate::compiler::families::assets::SourceAssetRegistry
            .route(
                "CommonModule",
                crate::compiler::families::assets::SourceAssetRole::Module,
            )
            .context("CommonModule file route is unavailable")?;
        let folder = crate::mssql_dump::root_family_folder(route.owner_family())
            .context("CommonModule source folder is unavailable")?;
        let prefix = format!("{folder}/");
        let suffix = format!("/{}", route.relative_path());
        let name = path
            .strip_prefix(&prefix)
            .and_then(|tail| tail.strip_suffix(&suffix));
        if !name.is_some_and(|name| {
            !name.is_empty() && name.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
        }) {
            return Err(RequestRefused(
                "SQL preflight currently supports one existing CommonModule file",
            )
            .into());
        }
        let source =
            SourceAdmission::prepare(source_dir, selection, verify, SourceLimits::default())?;
        check()?;
        let limits = readset::Limits::default();
        let storage =
            snapshot::observe_common_module_profile(client, database, limits, check.clone())?;
        let original =
            snapshot::capture(client, storage.snapshot_profile(), limits, check.clone())?;
        let edit = source.prepare_common_module_layout(
            &original,
            path,
            selected::Limits::default(),
            &check,
        )?;
        let policy = transaction::WritePolicy::existing_single_part(1024 * 1024, 4 * 1024 * 1024)?;
        let staged_rows = transaction::preflight_selected_parts(&original, &edit, &policy, &check)?;
        check()?;
        source.revalidate()?;
        let summary = CommonModulePreflight {
            backend: "mssql-common-module-configsave",
            target: database.into(),
            files: selection.files.clone(),
            missing_references: Vec::new(),
            ready: true,
            staged_rows,
        };
        Ok(Self {
            source,
            source_dir: source_dir.into(),
            original,
            edit,
            storage,
            policy,
            summary,
        })
    }
    pub fn summary(&self) -> &CommonModulePreflight {
        &self.summary
    }
    pub fn staged_rows(&self) -> usize {
        self.summary.staged_rows
    }

    /// Consumes the prepared operation. Callers must first consume the one-shot
    /// target/connection-bound token and obtain explicit user confirmation.
    /// Publication never compiles another path or rereads replacement contents.
    pub fn publish(
        self,
        client: &dyn crate::sql::SqlClient,
        current_database: &str,
        current_source_dir: &Path,
        check: readset::Check,
    ) -> transaction::Report {
        let before = (|| -> Result<()> {
            check()?;
            if current_database != self.storage.snapshot_profile().identity.database
                || current_source_dir != self.source_dir.as_path()
            {
                bail!("SQL preflight binding changed; prepare again");
            }
            self.source.revalidate()
        })();
        if let Err(error) = before {
            return transaction::Report {
                state: transaction::State::NotPublished,
                error: Some(error),
                cleanup_uncertain: false,
            };
        }
        transaction::stage(
            client,
            &self.original,
            &self.edit,
            self.storage.snapshot_profile(),
            &self.policy,
            check,
            &|| self.source.revalidate(),
        )
    }
}
