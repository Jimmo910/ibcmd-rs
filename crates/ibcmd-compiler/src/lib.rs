//! The source-to-storage compiler's coordinator, formerly the root's
//! `compiler` module. The crate mirrors the root's `compiler::` tree, so the
//! moved code keeps its `crate::compiler::…` paths; the root's `compiler`
//! module re-exports it.

pub mod compiler;
pub mod file_identity;
