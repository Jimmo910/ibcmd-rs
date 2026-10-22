# MCP adapter requirements

## Genuine protocol

The adapter SHALL use supported MCP stdio framing, version/capability handling,
tool schemas, cancellation and lifecycle from the official maintained SDK.
stdout SHALL contain only protocol messages; backend diagnostics SHALL NOT leak
credentials through tool content. Legacy editor JSON-RPC SHALL remain separate.

## Binding and authorization

An operator SHALL select the exact target and source root outside tool arguments.
The adapter SHALL reject masks, policy changes, replaced target roots, traversal,
symlinks and resource limit violations. Every call SHALL check authorization.
The read-only slice SHALL reject every import, and SHALL reject live targets
until stable database identity and shared write authorization are integrated.

## Shared operations

Tree/read/export SHALL use the common stored configuration engine and its existing
offline memory reader. The adapter SHALL NOT implement a second storage backend
or invoke native platform processes. Status/compare SHALL use the common object
comparator and include changed, base-only and local-only paths for selected
objects, including selected objects absent from the database. Source snapshots
SHALL be bounded, cancellable and confined; a file/directory census SHALL be
revalidated before returning comparison results. Concurrent source additions,
renames or replacements SHALL fail rather than silently report equality.
A cancelled request SHALL publish no result.

## Remaining completion gate

Issue #386 SHALL remain incomplete until a genuine MCP client changes a module
in a disposable native filedb, shared staging succeeds under explicit agent-write,
an external orchestrator applies normally, and native re-export matches source.
Mock write-policy tests SHALL NOT be represented as native acceptance.
