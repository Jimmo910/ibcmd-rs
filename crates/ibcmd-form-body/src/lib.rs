//! Inputs of the managed-form body reader (`src/mssql_dump/form_body.rs` in
//! the root crate, which is to move here): the per-export indexes and the
//! compatibility-mode rules its `FormParseContext` holds, the files a form's
//! items carry, and the dump timing report its entry points add their parts
//! to; and the code it reads besides them: the form body record
//! (`ParsedFormBodyBlob`), an adopted form's event block and base form slot,
//! the platform 8.5 layout's
//! down-conversion (`layout_8_5_1`) and the data-composition normalizer
//! (`dcs.rs`), which the root's template exporters share.
//!
//! Everything here moved from the root crate (`mssql_dump`'s `mod.rs`,
//! `refs.rs`, `dcs.rs`, `source_assets.rs`, `timing.rs`, `form_ref_index.rs`,
//! `form_extension.rs` and `form/layout_8_5_1.rs`, and `module_blob.rs`),
//! which still reaches every item by its old name and path through imports.
//! The bodies are unchanged; declarations and the fields the root reads or
//! fills are `pub`. The builders of these indexes read the root's metadata
//! rows and stay there, and so do the few methods that need a root type
//! (`MetadataFieldDeclarationIndex::written_as` and the sums of
//! `MssqlDumpTimingReport`) or would widen the root's public API
//! (`FormBodyRevision::parse`), as root traits. The tests that decode a
//! template through the root's codec stay in the root's `dcs.rs`.
//!
//! The code came from the physical-adapter guard's slice, so the guard scans
//! every non-test `*.rs` file under this crate's `src/`
//! (`$GuardedCrateDirectories` in `tools/validate-physical-adapter-policy.ps1`)
//! and their literals stay in the guard's baseline. A file added here is in
//! the slice at once and fails the guard until its record is reviewed.

mod assets;
mod blob;
mod compatibility;
mod dcs;
mod declarations;
mod form_extension;
mod form_ref_index;
mod indexes;
pub mod layout_8_5_1;
mod timing;

pub use assets::FormItemAsset;
pub use blob::{FormBodyRevision, ParsedFormBodyBlob};
pub use compatibility::{
    FormCompatibility, compatibility_mode_spells_any_ref, forms_mark_no_main_table_default_picture,
    forms_write_usual_group_behavior,
};
pub use dcs::{
    ANY_IB_REF_TYPE_ID, CanonicalDcsSettingsAdapterError, CanonicalDcsSettingsContext,
    CanonicalDcsSettingsInput, CanonicalFormServerStateFragment, DcsSettingsCanonicalizeError,
    DcsSettingsChild, DcsTemplateNormalizeError, FormListSettingsChildKind,
    FormListSettingsChildTransliteration, canonicalize_form_server_state_conditional_appearances,
    data_composition_type_id_xml, emit_canonical_dcs_settings_children,
    emit_canonical_dcs_settings_parts,
    normalize_data_composition_schema_template_documents_with_profiles,
    respell_form_server_state_namespaces,
    transliterate_form_attributes_conditional_appearance_document,
    transliterate_form_list_settings_child_document,
    transliterate_form_list_settings_group_items_document,
};
pub use declarations::{
    MetadataCommonAttributeContent, MetadataConstantDeclaration, MetadataFieldDeclarationIndex,
    MetadataTableStandardAttributes,
};
pub use form_extension::{
    CALL_TYPES, FormEventBinding, adopted_base_record_slot, parse_form_event_block,
};
pub use form_ref_index::FormObjectRefIndex;
pub use indexes::{
    DcsTypeIndex, DcsTypeResolution, InformationRegisterFieldReference,
    InformationRegisterFieldReferenceIndex, InformationRegisterMasterDimensionIndex,
    MetadataCommandReference,
};
pub use timing::MssqlDumpTimingReport;
