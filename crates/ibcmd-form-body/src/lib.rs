//! Inputs of the managed-form body reader (`src/mssql_dump/form_body.rs` in
//! the root crate, which is to move here): the per-export indexes and the
//! compatibility-mode rules its `FormParseContext` holds, the files a form's
//! items carry, and the dump timing report its entry points add their parts
//! to.
//!
//! Everything here moved from the root crate's `mssql_dump` (`mod.rs`,
//! `refs.rs`, `dcs.rs`, `source_assets.rs`, `timing.rs` and
//! `form_ref_index.rs`), which still reaches every item by its old name
//! through imports. The bodies are unchanged; declarations and the fields
//! the root's builders fill are `pub`. The builders of these indexes read the
//! root's metadata rows and stay there, and so do the few methods that need a
//! root type (`MetadataFieldDeclarationIndex::written_as` and the sums of
//! `MssqlDumpTimingReport`), as root traits.
//!
//! The code came from the physical-adapter guard's slice, so the guard scans
//! every non-test `*.rs` file under this crate's `src/`
//! (`$GuardedCrateDirectories` in `tools/validate-physical-adapter-policy.ps1`)
//! and their literals stay in the guard's baseline. A file added here is in
//! the slice at once and fails the guard until its record is reviewed.

mod assets;
mod compatibility;
mod declarations;
mod form_ref_index;
mod indexes;
mod timing;

pub use assets::FormItemAsset;
pub use compatibility::{
    FormCompatibility, compatibility_mode_spells_any_ref, forms_mark_no_main_table_default_picture,
    forms_write_usual_group_behavior,
};
pub use declarations::{
    MetadataCommonAttributeContent, MetadataConstantDeclaration, MetadataFieldDeclarationIndex,
    MetadataTableStandardAttributes,
};
pub use form_ref_index::FormObjectRefIndex;
pub use indexes::{
    DcsTypeIndex, DcsTypeResolution, InformationRegisterFieldReference,
    InformationRegisterFieldReferenceIndex, InformationRegisterMasterDimensionIndex,
    MetadataCommandReference,
};
pub use timing::MssqlDumpTimingReport;
