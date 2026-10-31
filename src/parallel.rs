//! The process-wide worker pools. Their one owner, with the pool and
//! requested-worker state, is the `ibcmd-parallel` crate; this module keeps
//! the root paths.

pub use ibcmd_parallel::{bounded_worker_count, io_bound_worker_count, request_workers};
pub(crate) use ibcmd_parallel::{install, install_io_bound, install_memory_bound_or_inline};
