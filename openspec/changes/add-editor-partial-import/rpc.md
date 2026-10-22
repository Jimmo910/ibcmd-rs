# Editor import protocol 1.2

Existing 1.1 clients are accepted. SQL objects/import retains its previous contract. rows-dir stays read-only. New verified editor import currently supports Unix file-db connections with measured collation; Windows refuses the new CAS path explicitly.

Prepare (exactly one selection mode):

```json
{"method":"objects/import/prepare","params":{"infobase":"lab","sourceDir":"/workspace/src","files":["Documents/Example/Ext/ObjectModule.bsl"]}}
```

`objects:["Document.Example"]` selects existing objects, including their owned files. `changed:true` selects local differences only when their synchronized ConfigDumpInfo row identity/name/version still agrees with the effective target. Missing/stale baseline fails before publication. Import does not automatically rewrite the source inventory: refresh it after synchronization, or use explicit file selection when reviewing a conflict.

Response: `{plan, preflight:{target, files, ready, missingReferences}}`. Missing references include an optional suggestedObject. Adding dependencies is a separate user-confirmed scope change and requires a fresh preflight; new objects remain unsupported. A plan with unresolved references cannot publish.

Publish:

```json
{"method":"objects/import","params":{"infobase":"lab","sourceDir":"/workspace/src","plan":"opaque-preflight-token","confirm":true}}
```

Without confirmation there is no write and the plan remains usable. Tokens are one-use, bound to the connection generation, canonical target inode/content and source census. A reconnect invalidates all pending tokens. Source or target changes require a fresh preflight. There is one pending plan per connection, at most four per server.

VSCode commands: `ibcmd.importSelected` accepts file Uri selections or #380 Row values `{object,uri}`; `ibcmd.importChanged` validates Explorer URI context and otherwise uses the current connection. `ibcmd.sourceDirectory` (default `.`) is workspace-relative. File/object selection outside that source tree and dirty files are refused. Source/connection/settings are rechecked after each asynchronous confirmation. Dependency consent and final file-list consent are separate; cancellation is forwarded through $/cancelRequest. Early known validation messages are typed and displayed; unknown connection errors retain existing sanitization.

No production extension code launches native ibcmd. Applying the stage belongs to #384 and the platform. `config/pending` remains unavailable for file-db in this slice; the UI reports pending unknown rather than asserting no pending changes.

## MCP integration boundary

The common guard API supplies consumed descriptor metadata and a final publication policy gate. MCP must additionally keep its materialization/time budgets throughout preparation and validate startup identity before parsing a substituted database. The separately developed bounded reader f2a05c34 is not implicitly forwarded to WriteSession by this commit. A consumer must keep agent import disabled until that bounded write-preflight route is integrated and reviewed; the editor's trusted local file-db acceptance does not establish the MCP untrusted-input budget gate.
