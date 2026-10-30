//! Profile-gated codecs for native bodies that are not metadata-family rows:
//! the ones the coordinator compiles through. The root's `compiler::bodies`
//! keeps the others and re-exports these.

// The body-profile selection is `ibcmd-compiler-families`'s; the codecs
// here keep naming it from this module.
pub use ibcmd_compiler_families::compiler::bodies::BodyProfileError;
pub(crate) use ibcmd_compiler_families::compiler::bodies::SelectedBodyProfile;

pub mod form;
// `TemplateKind` and its codec error, without the DCS and MXL codecs that the
// root's `template` compiles bodies through.
pub mod template_kind;
