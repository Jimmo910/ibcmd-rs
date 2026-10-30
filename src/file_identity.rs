//! Physical identity of an opened file, including hard-link and symlink
//! aliases: `ibcmd-compiler`'s `file_identity`, which the external intake
//! reads; the save command reads it here.

pub(crate) use ibcmd_compiler::file_identity::file_identity;
