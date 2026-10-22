# Protocol and authorization design

Protocol research verified 2026-10-22:

- [Current specification](https://modelcontextprotocol.io/specification/2026-07-28)
- [stdio binding](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio)
- [Versioning](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning)
- [Official Rust SDK rmcp 3.6.0](https://docs.rs/crate/rmcp/3.6.0)

Pin rmcp exactly to 3.6.0 (MSRV 1.88, tested with Rust1.95). Use its supported
AsyncRwTransport over stdio, newline-delimited MCP JSON-RPC. The SDK owns modern
server/discover/per-request metadata and legacy initialize/initialized, version
and capability processing, notifications/cancelled, errors and disconnects.
Only tools capability is advertised. No listener or native platform subprocess.

The operator launches:

```
ibcmd-rs mcp --settings /operator/ibcmd-rs.toml --policy /operator/mcp.toml
```

The explicit common settings file has one fixed offline database, e.g.:

```toml
[[database]]
name = "demo"
platform = "8.3.27"
rows-dir = "/approved/canonical/rows"
```

The policy may reside in `[mcp]` of the same `--settings` file (default),
or an explicit separate `--policy` file:

```toml
[mcp]
database = "demo"
source-root = "/approved/canonical/source"
```

Common-file schema from dependency 644684f2: same `[mcp]` fields in ibcmd-rs.toml,
plus `[[database]] agent-write = true` for one exact fixed target, false by
default; true forbidden for masks. Adapter must not infer write permission from
client annotations, tool parameters or a model's confirmation. The first slice
never writes even if a policy claims permission. Unknown policy fields fail.

Every call checks pinned policy bytes, common settings bytes, source root identity
and offline rows root identity. The selected entry must match the startup bytes,
not just a second read by the shared parser. No model-supplied server, id, auth,
password, settings, source root or output destination. No automatic reconnect to
a replaced target. SQL is disabled until immutable live identity is available.
The operator must protect these files with OS permissions independently of agent
source edits; the adapter does not authenticate a model as the database operator.

Read offline rows through directory-relative NOFOLLOW descriptors into a bounded
immutable snapshot. Feed that snapshot into existing OfflineRows::from_memory,
ConfigTable and export sinks, preserving a single backend. Source comparisons
also use directory-relative NOFOLLOW reads, reject traversal, absolute paths,
backslashes, empty components and special files. Snapshot rows are limited to
128MiB and100000 parts; inputs to32 objects/32KiB, transport lines64KiB, results
1MiB. Resource bounds are enforced before publication; native engine resource
limits still govern decoding. One admitted worker avoids an unbounded blocking
queue. Cancelled futures set the shared engine flag and suppress publication.
Backend error details are redacted from tool content; research environment
filesystem overrides are refused at startup.

`tree` lists children; `read` returns one exported file; `export` returns selected
files in memory as UTF-8/base64. `status` and `compare` currently report equality
for exported paths only and explicitly label `comparisonScope=exported-files-only`.
They do not establish complete object equality when local-only paths exist.
`import` returns an explicit error with no publication. Full shared status
semantics and staging are integration work, not simulated acceptance.
