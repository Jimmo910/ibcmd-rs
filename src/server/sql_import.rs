//! Bounded source admission for SQL editor preparation.
//!
//! Source admission, snapshot compilers and a library-only guarded transaction
//! primitive are separate from user dispatch. No RPC route calls the transaction;
//! native writer/profile and end-to-end acceptance remain required before wiring.
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
        check()?;
        if self.selected.len() != 1 {
            bail!("SQL module preparation requires one explicit admitted file");
        }
        let bytes = self
            .bytes(selected_path)
            .context("SQL module file is outside the admitted selection")?;
        self.revalidate()?;
        let edit =
            selected::prepare_module_file(snapshot, selected_path, bytes, profile, limits, check)?;
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
