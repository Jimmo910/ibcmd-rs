//! What the form body reader (`form_body.rs`) is handed by the rest of the
//! root instead of reading it: the `module_blob` readers of the Form
//! Attributes conditional-appearance tail ([`ROOT_FORM_BODY_HOST`]) and the
//! state of the extension export running now
//! ([`active_form_extension_export`]). `FormParseContext::new` takes both;
//! nothing else in `form_body.rs` names `module_blob` or `extension`.

use super::form_body::{FormBodyHost, FormExtensionExport};

/// The root's readers of the Form Attributes tail's physical layout.
pub(super) const ROOT_FORM_BODY_HOST: FormBodyHost = FormBodyHost {
    conditional_appearance_tail_storage_document:
        crate::module_blob::form_attributes_conditional_appearance_tail_storage_document,
    parse_conditional_appearance_tail:
        crate::module_blob::parse_form_attributes_conditional_appearance_tail,
};

/// The extension export running now, as a form reads it; `None` in an
/// ordinary export. The extended configuration's compatibility mode is read
/// once, right after the export activates its context
/// (`extension::export_extension_image_to_source`), so this is a cached read.
pub(super) fn active_form_extension_export() -> Option<FormExtensionExport> {
    super::extension::active().map(|extension| FormExtensionExport {
        extended_compatibility_mode: extension.extended_compatibility_mode().map(str::to_owned),
    })
}
