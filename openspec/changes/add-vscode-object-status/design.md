# Native versions and independent local-file evidence

`objects/status` keeps its original byte-only request/response. Optional
`versionBaselineDir` adds native DB evidence through #362's shared
`native_config_dump_info` / `ConfigDumpInfoSnapshot` helper. `fullRequired`, typed
`nativeChanges` and `databaseSnapshot` are preserved. Per-row
`databaseVersionStatus` contains basis, exact tree object, objectVersion,
baselineIdentity, databaseSnapshot, conclusive and optional changed.

The engine reads the native inventory before/after tree/byte comparison and checks
reference bytes before/after. The client makes both source and reference calls in
native mode, compares the complete native snapshot, per-object cache tokens and
reference SHA, then combines the native changed flag with independent local/reference
bytes. Tokens/hashes are freshness identities, never native configVersion values.
The byte-difference union remains solely a way to discover local/reference paths.
Native mode is default; byte preview is explicit and never an automatic fallback.

The narrow adapter requires row identity and exact canonical object name. Root
Configuration and measured CommonModule.Module are explicit cases. Unknown row
roles, conflicting identities and incomplete coverage are inconclusive; no nearest
ancestor mapping. Native fullRequired makes common-object evidence inconclusive.
DB-only/file-only existence remains separate from native version change reporting.

Filedb uses the shared effective CONFIG+completed CONFIGSAVE backend. MSSQL native
mode is gated before files/SQL: its active byte/tree view does not yet match the
main-state native helper. Rows and the legacy byte API remain compatible. Further
role/view support must come from common engine evidence, not a parallel adapter.

Generation invalidation and AbortController reject cancelled/stale requests after
refresh, selection, settings, crash or workspace removal. Folder references are
read-only. Git refs are pinned and materialized in an explicit external cache.
No native executable or shell is invoked by production code.

See version-contract-proposal.md for historical rationale and its implementation
update, plus evidence/native-integration-20261022.md for real six-state acceptance.
Parent review/PR gate remains; no complete ERP browser/performance or GUI claims.
