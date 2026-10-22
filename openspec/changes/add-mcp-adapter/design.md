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
client annotations, tool parameters or a model's confirmation. The current slice
never writes even if a policy claims permission. Unknown policy fields fail.

Every call checks pinned policy bytes, common settings bytes, source root identity
and offline rows root identity. The selected entry must match the startup bytes,
not just a second read by the shared parser. For filedb, pin the data file inode
and pass a fresh NOFOLLOW descriptor with the same device/inode directly to
FileDb::from_file; the existing shared effective CONFIG+CONFIGSAVE reader returns
a snapshot. Do not reopen the target through Session after authorization. No model-supplied server, id, auth,
password, settings, source root or output destination. No automatic reconnect to
a replaced target. SQL is disabled until immutable live identity is available. Local filedb reads
use shared dependencies 644684f2 and 571e6a13 (integrated as ea8097f5/7c911837).
The operator must protect these files with OS permissions independently of agent
source edits; the adapter does not authenticate a model as the database operator.

Read offline rows through directory-relative NOFOLLOW descriptors into a bounded
immutable snapshot. Feed that snapshot into existing OfflineRows::from_memory,
ConfigTable and export sinks, preserving a single backend. Source comparisons
also use directory-relative NOFOLLOW reads, reject traversal, absolute paths,
backslashes, empty components and special files. Offline snapshot rows are limited before accumulation to
128 MiB and 100000 parts; filedb input files are limited to128 MiB. Filedb reads use FileDb::from_file_with_limits: cumulative limits are charged
before object buffers and copied blob/metadata values are allocated, including
aliases and repeated table reads. Budget: 100000 descriptors/live records,
128 MiB copied row binary, 256 MiB object/metadata buffers; each metadata chain
is capped at1 MiB in the bounded API. Cancellation is checked in initialization,
page reads, metadata parsing, record loops and blob-block walks. Existing
unbounded APIs retain their previous limits. These counters bound major input
buffers/copies; they do not claim an exact process heap ceiling. Tool inputs are
limited to32 objects/32 KiB, transport lines64 KiB, results1 MiB. Resource bounds are enforced before publication; native engine resource
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


## Validation evidence (2026-10-22)

Rust1.95, explicit PATH/RUSTC, jobs1, external build/issue-386 and lab-env caches.
`cargo test --test mcp_adapter --test editor_server --test editor_file_db` passed:
3 real MCP subprocess/client tests, 6 existing editor protocol tests, 6 shared
filedb contract tests. MCP fixture exports match existing cf export bytes. Both
2025-11-25 initialize and2026-07-28 discovery are exercised, including a genuine
SDK cancellation notification. Policy changes, target replacements, override
parameters, traversal and symlinks are refused. Real filedb MCP read/export and
compare match tiny-xml, with identical database bytes and no backup created.

External native READ oracle:
`IBCMD_MCP_NATIVE=/opt/1cv8/8.5.1.1592/ibcmd cargo test --test mcp_adapter real_mcp_client_reads_disposable_filedb_without_mutation -- --exact`
passed. The test creates its own disposable DB and native --data/--temp/output
under external TMPDIR, and compares native-exported module bytes with the actual
MCP result/source. Native execution exists only in this test orchestrator.
Logs: lab logs/issue-386-filedb-mcp.log and logs/issue-386-native-read.log.
This is read evidence; it does not satisfy the write/apply roundtrip gate.

Independent security reviews found and fixed parser separation, rows snapshot
confinement, worker admission, server identity, FIFO reads, part-count limits and
strict policy fields. Descriptor filedb review confirms consumed identity and
snapshot routing; malformed-file materialization was subsequently closed with constructor/row
budget tests and cancellation checks, including malformed native-fixture copies
with repeated blob and descriptor aliases.
