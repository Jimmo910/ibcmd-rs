# ibcmd-rs Connections for VS Code

Minimal connection UI for [#378](https://github.com/Untru/ibcmd-rs/issues/378).
Requires VS Code 1.95+ and an `ibcmd-rs` executable speaking editor protocol **1.1**
(this checkout). A 1.0 server is rejected before a connection is attempted.

## Use

1. Compile with `npm ci && npm run compile` in this directory. Launch this directory
   as an extension development host (example below). Packaging/Marketplace release
   is separate work.
2. Open a **trusted, local workspace folder**. Set **ibcmd: Executable** in user or
   remote-machine settings to the executable path; this setting is machine scoped,
   so workspace content cannot select a program to execute. No shell is used.
3. Run **ibcmd-rs: Add Connection**. Supply a friendly name, SQL Server, database,
   platform and Windows or SQL authentication. SQL passwords go to VS Code
   `SecretStorage`; the prompt is masked. Cancel leaves the connection unchanged.
4. Use **Bind Workspace Connections** to select several existing databases for
   a folder, then **Select Workspace Connection**, or click the status bar, to
   switch. Multi-root workspaces first ask which folder to use. Fixed database
   names are supported; existing wildcard entries can be edited into fixed names.
5. **Reload Connections and Refresh Status** rereads the settings, reconnects the
   selected bound database and requests pending state. The displayed pending state
   is a snapshot, not a background monitor. **Forget Connection Password** deletes
   the selected login's saved password and closes the current session.

Windows integrated authentication uses the account running the server and needs
Windows. SQL authentication requires a user. This client always chooses auth
explicitly and never inherits a global SQL login/password for that connection.
An actual SQL query must succeed before the status shows a connected database.
No live SQL or Windows login was used to validate this initial client.

## Shared configuration and secrets

Database `server`, `name`, `platform` (and existing `rows-dir`) remain in the normal
`ibcmd-rs.toml` format. Add/Edit writes the **workspace folder's** file; an inherited
entry is copied into that file. `settings show` and the editor read the usual
executable/AppData/workspace settings layers. The editor ignores `IBCMD_RS_CONFIG`
and removes `IBCMD_DB_PSW` from the server environment, so it uses workspace
settings and SecretStorage. To compare CLI output, unset an explicit
`IBCMD_RS_CONFIG` override or point it at the same workspace file.

Friendly names, auth mode, SQL user and bindings are editor preferences in
`.vscode/settings.json`, scoped to the folder. They do not require new TOML keys
and remain compatible with older CLI versions:

```json
{
  "ibcmd.connections": ["sql01/demo", "sql01/test"],
  "ibcmd.connectionMetadata": {
    "sql01/demo": { "label": "Development", "auth": "sql", "user": "developer" },
    "sql01/test": { "label": "Test", "auth": "windows" }
  }
}
```

Secret keys include the folder URI, database ID and SQL user; credentials are
not silently reused between projects, users or servers. Editing a login replaces
its password; changing the identity removes the previous secret after saving.
No password is placed in arguments, TOML, workspace settings, error messages or
extension output. Server stderr, `log` notifications and server error text are
not forwarded. The only password-bearing transport is the child process's stdin.
VS Code manages encryption/backing storage for SecretStorage.

Edit preserves comments and other keys by replacing only AST value spans.
Settings are saved with a flushed sibling temporary file and atomic rename.
A sibling exclusive lock serializes this client's writes, and exact snapshot
checks reject concurrent disk changes; dirty TOML editor buffers are rejected.
Inline `database = [{...}]` syntax can be read but Add/Edit refuses it rather than
rewriting it or dropping comments: use `[[database]]` tables. Existing duplicate
IDs must be resolved manually. If a process is forcibly killed during a save,
inspect and move its stale `.ibcmd-lock`/`.ibcmd-*.tmp` artifacts aside before retrying.
An unrelated writer that ignores the lock can still race in the tiny interval
between the final comparison and rename: portable filesystems provide no
compare-and-swap replacement. Avoid simultaneous external writes to this file.
TOML and VS Code settings/SecretStorage are separate saves, not a cross-file
transaction; after an interrupted save, inspect the TOML and rerun Edit/Bind.

Settings changes close the current server. Re-select or Reload to initialize
fresh settings. Switching/reloading disposes the old process because the protocol
has no disconnect method. Late answers cannot replace the current selection.
Pending errors, inconclusive verdicts and offline rows sources show **pending
unknown**, never “no pending changes”. A crashed server shows **disconnected**.

## Development and checks

All caches and host data can be placed beneath one chosen directory. On the
ibcmd-rs lab Mac, source `lab-env.sh`, use `CARGO_TARGET_DIR=$LAB/build/issue-378`,
and use an `IBCMD_TEST_ROOT` inside `$LAB/tmp`. No global npm extension installation
or VS Code profile change is needed.

```sh
# From the repository root; build the binary and generate offline rows from
# committed CF fixtures (no SQL Server).
cargo test --locked --test editor_server --no-default-features
export IBCMD_TEST_BINARY="$CARGO_TARGET_DIR/debug/ibcmd-rs"
export IBCMD_TEST_ROWS="$TMPDIR/ibcmd-external-editor-server-protocol/rows/demo"
export IBCMD_TEST_ROOT="$TMPDIR/ibcmd-vscode-tests"
cd editors/vscode
npm ci
npm test
npm run format:check
# Use installed VS Code, or omit this to download a test host into IBCMD_TEST_ROOT.
export VSCODE_EXECUTABLE_PATH="/Applications/Visual Studio Code.app/Contents/MacOS/Code"
npm run test:host
```

`npm test` covers TOML preservation/conflicts, UTF-8 framing, cancellation,
timeouts, crash, error redaction, generations and pending verdicts. With
`IBCMD_TEST_BINARY`, it also checks actual `settings show` and `serve --stdio`.
The extension-host suite drives Add/Edit/Bind/Select/Forget flows with controlled
UI responses, actual SecretStorage and CLI readback, plus a real offline
connection. Synthetic extension-host fixtures currently require POSIX; Windows
is covered by CLI/RPC and unit CI, not by a local Windows host run. Its **explicitly synthetic** server covers pending/error/crash and
switch races; these tests are not evidence of a SQL login. The host gets isolated
`user-data-dir`, `extensions-dir`, crash and temporary directories. Test evidence
is retained in the selected temporary root for inspection.

For an isolated manual development host after compilation:

```sh
code --extensionDevelopmentPath="$PWD" \
  --user-data-dir="$IBCMD_TEST_ROOT/manual-user-data" \
  --extensions-dir="$IBCMD_TEST_ROOT/manual-extensions" /path/to/workspace
```

Object trees, export/import/apply UI, VSIX/Marketplace delivery and live SQL
verification are outside this connection-only candidate.

## Object status preview (#380)

The Explorer **ibcmd-rs Objects** view is a minimal flat list of object full names,
not the full metadata browser planned in #379. Select a connection, configure the
source/baseline for that workspace folder, then run **Refresh Object Status**.
**Reload Connections and Refresh Status** also refreshes objects after reconnecting.
The view toolbar toggles **Changed Objects Only** (unknown remains visible).
The same status decorates source files in Explorer, with the most specific nested
object taking precedence.

```json
{
  "ibcmd.sourceDirectory": "src",
  "ibcmd.statusBaselineDirectory": "/external/reference-export"
}
```

The baseline must be a separate, readable export from the last synchronization;
it is never updated by refresh. Alternatively set `ibcmd.statusGitRef` to a commit,
branch or tag and `ibcmd.statusCacheDirectory` to an absolute cache folder (in the
lab, under the external lab root). Git takes precedence over the baseline folder.
The ref resolves to a fixed tree before reading committed blobs. Only the source
subtree is materialized, without checkout, hooks, shell or native ibcmd invocation.
Snapshots are retained for inspection and user-controlled Trash cleanup.
A Git baseline lacking a tracked export cannot establish a synchronization history.

| Label | Badge | Evidence |
| --- | --- | --- |
| Совпадает | = | DB and local both equal baseline |
| Изменён в базе | Б | DB differs from baseline; local equals baseline |
| Изменён в файлах | Ф | DB equals baseline; local differs |
| Изменён в базе и файлах | БФ | Both differ from baseline, including identical concurrent edits |
| Только в базе | +Б | RPC says object has no local files |
| Только в файлах | +Ф | RPC says object is absent in DB |
| Статус неизвестен | ? | No baseline/direction evidence, DB versions changed between comparisons, or incomplete evidence |

The current RPC compares exported **bytes**. For common objects the client asks
`objects/status` for source and baseline, verifies the object's DB `version` agrees
between responses, and compares local/baseline bytes for the union of differing
paths. Additions/deletions and binary files are included. Missing source/baseline
folders, symlink paths, unsaved source editors, malformed replies, failed or cancelled
refresh clear old decorations and show unknown instead of a clean result.
Connection/configuration/workspace changes invalidate all outstanding status work;
only the selected workspace folder receives decorations.

### Acceptance boundary

This preview is tested on committed `rows-dir` fixtures and a synthetic six-state
extension-host fixture. It is **not yet native export-status parity**: the existing
RPC does not expose the `ConfigDumpInfo.xml`/native version comparison. The shared
#382 READ backend has been integrated from `644684f2`; it does not yet provide write/pending operations. No
duplicate filedb adapter is provided here.
Issue #380 remains open until the native #362 version contract is agreed and
exposed through RPC, and a disposable filedb receives a platform module edit plus a
separate local file edit with matching measured status in the extension host.

Checks: `npm test`, `npm run format:check`, `npm run test:host`; the host test covers
real rows-dir export/comparison plus explicitly synthetic six states, filter,
Explorer badges, error clearing, cancellation and selection races. Use a unique
`TMPDIR` under the lab root for Rust fixture tests: their shared upstream helper
clears/reuses fixed fixture directory names. Keep preserved scratch artifacts out
of those names.

Actual filedb READ/extension-host evidence after integration is documented in
`openspec/changes/add-vscode-object-status/evidence/native-read-20261022.md`.
The measured native-version/byte divergence is a remaining acceptance blocker.
For an optional prepared native-mutated lab run, set `IBCMD_TEST_FILEDB_ROOT` to
its disposable run root before `npm run test:host`; this additionally checks actual
filedb UI badges and records `ui-evidence.json` with `nativeVersionParity: false`.
The tree description and tooltips explicitly identify this as a byte preview.
