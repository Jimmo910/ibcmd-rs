//! PostgreSQL client API used by the root infobase adapters.
//!
//! Transport, authentication and protocol codecs live in the portable,
//! strictly linted workspace crate; infobase queries and snapshots stay here.

pub use ibcmd_postgres::{PgConnection, PgTarget, decode_bytea};
