//! What the packers of a configuration's stored bodies read of its source
//! tree: a source file's bytes ([`SourceBytes`]), and the answers the source
//! context memoises for the interface and help writers
//! ([`InterfaceResolutionMemo`], [`HelpReference`]).
//!
//! These moved from the `ibcmd-metadata-blob` crate, which re-exports them.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// A source file's bytes: shared with the preloaded store, or read for the
/// caller.
pub enum SourceBytes {
    Shared(Arc<Vec<u8>>),
    Owned(Vec<u8>),
}

impl std::ops::Deref for SourceBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            Self::Shared(bytes) => bytes,
            Self::Owned(bytes) => bytes,
        }
    }
}

impl SourceBytes {
    #[doc(hidden)]
    pub fn into_vec(self) -> Vec<u8> {
        match self {
            Self::Shared(bytes) => bytes.as_ref().clone(),
            Self::Owned(bytes) => bytes,
        }
    }
}

/// What resolving names reads more than once, kept for the life of the
/// source context: the class and uuid of a metadata file, and the commands an
/// owner declares.
#[derive(Debug, Default)]
#[doc(hidden)]
pub struct InterfaceResolutionMemo {
    #[doc(hidden)]
    pub headers: BTreeMap<PathBuf, Option<(String, String)>>,
    #[doc(hidden)]
    pub commands: BTreeMap<PathBuf, Option<Arc<BTreeMap<String, String>>>>,
}

/// What a readable reference names in the source tree.
#[derive(Clone, Debug)]
#[doc(hidden)]
pub enum HelpReference {
    Resolved(String),
    /// No file of the tree declares it: the configuration has no such object.
    Absent,
}
