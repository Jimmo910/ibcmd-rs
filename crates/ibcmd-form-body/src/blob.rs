//! A form body record as the root's `module_blob` unpacks it (from
//! `src/module_blob.rs`, which re-exports both types under their old public
//! paths and keeps the revision reader, `FormBodyRevision::parse`, as a
//! root trait).

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedFormBodyBlob {
    /// The container revision the record declared in its own first slot.
    pub revision: FormBodyRevision,
    pub layout: String,
    pub module_text: String,
    pub trailing: Vec<String>,
    pub trailing_fields: usize,
}

/// The container revision a Form body declares in its own first slot.
///
/// Both admitted revisions carry the identical ten-slot container -- marker,
/// layout, module text, five blocks and two scalars. Measured over every form
/// of all eight stand corpora (22,646 forms): revision `4` on 22,535 managed
/// forms of all eight, revision `3` on 102 managed forms of ERP УХ 3.2.12.6
/// only, and no other value anywhere. Layout revision `49` occurs under both
/// container revisions, layout `50` only under `4`. See
/// `docs/evidence/uh-form-body-container-revision-20260826.md`.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FormBodyRevision {
    V3,
    V4,
}

impl FormBodyRevision {
    pub const fn declared_marker(self) -> &'static str {
        match self {
            Self::V3 => "3",
            Self::V4 => "4",
        }
    }
}
