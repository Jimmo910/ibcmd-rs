# Shared backend API (first dependency commit)

`settings::DatabaseBinding` adds `db_path: Option<PathBuf>` and `agent_write: bool` (default false). TOML keys `db-path` and `agent-write`; file path resolves against the settings file. File entries reject server/rows-dir/masks; agent-write rejects masks. `[mcp]` is accepted as opaque TOML by the common parser; MCP owns strict policy validation.

`server::infobases::EntryParams` adds `db_path: Option<PathBuf>` (JSON `dbPath`). `Entry` adds the same response field and uses `source = "file-db"`. `Session::open` revalidates public Entry invariants and canonicalizes/opens the actual data file without SQL or inherited credentials.

`stored_objects::ConfigSource::FileDb(PathBuf)` is a real file database. `StoredConfiguration::{children,resolve,export,read_file,status}` use the existing effective CONFIG+CONFIGSAVE export path. No native platform process is spawned.

This first dependency commit provides READ support only. `objects/import` and `config/pending` refuse file-db explicitly; the partial write API follows separately. Consumers must not fall back to SQL on file-db. `agent_write` is schema, not permission enforcement. MCP must resolve an exact settings entry and canonical target; the general settings database matcher permits server wildcards and cannot alone authorize writes. Editor initialize entries cannot grant agent permission.

Settings/server consumer file ownership remains with #382 until this commit is integrated. RowsSnapshot additions from #386 are additive and need merge reconciliation at ConfigSource matches.

## Evidence

Contract test was run before implementation: failed on unknown `dbPath`. Five integration tests then passed: real 1CD tree/read/status with byte-identical target and no backup, legacy SQL/rows-dir routing, conflicting selectors, opt-in default/mask refusal, and public Session invariant revalidation. Independent reviewer found the latter validation gap; fixed with regression test. Build uses explicit official Rust 1.95, two jobs, lab-only target/temp/cache.
