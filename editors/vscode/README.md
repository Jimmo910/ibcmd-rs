# ibcmd-rs Connections for VS Code

Minimal connection UI for [#378](https://github.com/Untru/ibcmd-rs/issues/378).
Requires VS Code 1.95+ and an `ibcmd-rs` executable speaking editor protocol **1.3**
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
export IBCMD_TEST_ROWS="${TMPDIR:-/tmp}/ibcmd-external-editor-server-protocol/rows/demo"
export IBCMD_TEST_ROOT="${TMPDIR:-/tmp}/ibcmd-vscode-tests"
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

Edit Connection supports SQL and read-only rows-directory entries. An omitted server
stays omitted so normal `db-server`/environment resolution still applies. Copying an
inherited rows-directory entry keeps its resolved source directory. File databases
and unknown source types are excluded from this editor; edit their settings using
the appropriate source-specific tooling instead.

### Selected and changed import (#382)

For a Unix file database, add a fixed `[[database]]` entry with `db-path` in
workspace TOML and select that connection. `ibcmd.sourceDirectory` defaults to
`.` and may name a configuration source folder inside the workspace. Explorer
and editor context menus provide «Загрузить в базу»; «Загрузить изменённые» is
also available. The #380 object-view hook accepts its `{object, uri}` rows.

Import prepares and verifies a complete effective CONFIG/CONFIGSAVE baseline,
then asks for confirmation before publishing. Dependency suggestions require
separate consent and another check. Unsaved files, switched connections,
changed source/target and unsupported changes fail before publication. New
objects/deletions and unsupported file collations are refused.

Changed import requires synchronized `ConfigDumpInfo.xml` versions; it refuses
stale database/staging versions rather than reverting them. Refresh the export
baseline after synchronization, or explicitly select files when reviewing a
conflict. Successful import leaves CONFIGSAVE staged; apply is available through the separate
**ibcmd-rs: Apply CONFIGSAVE** command (#384). File-db pending state is refreshed
after import and apply.

For local evidence collection, set `IBCMD_TEST_KEEP_FIXTURES=1` before the Rust
file-db, partial-import and source-traversal tests. Their newly owned fixture
directories are then retained, including after assertion failure. Without this
flag, disposable CI fixtures are cleaned on scope exit. This test-only switch
does not change production temporary-file cleanup or recovery artifacts.


## Apply CONFIGSAVE (#384)

After importing, the status bar refreshes the CONFIGSAVE pending indication.
Use **ibcmd-rs: Apply CONFIGSAVE** from the command palette. The confirmation
names the infobase, backend and exclusive mode. The `ibcmd-rs Apply` Output
channel reports start, completion and uncertain outcomes without forwarding
configuration data, native output, server error chains or credentials.

- SQL connections call `config/apply` using the existing own #343 backend.
  Exclusive session checks and the existing structural gate remain enforced;
  this editor route does not enable restructuring, assumed exclusivity,
  session termination or dynamic mode. Recovery artifacts are retained in a
  fresh `.ibcmd-recovery/<uuid>` directory inside the connected workspace.
- File databases use native `ibcmd` **from the extension**, after consent.
  Set machine-scoped `ibcmd.nativeExecutables`, mapping the bound platform
  version (for example `8.5.1`) to an absolute executable path from that
  platform installation. Verify that mapping before use. The server only
  returns the connected file database's descriptor and never launches native
  tools. Native `--data` and `--temp` are fresh subdirectories under the
  workspace's `.ibcmd-recovery`, retained for inspection.
- Stored-row folders cannot apply. The command exposes exclusive mode only;
  dynamic compatibility is tracked separately in #347.

Keep a usable backup and close file-database sessions before consent. Cancel
before publication stops the request at a checkpoint; native cancellation
terminates the process. Once SQL execution or native publication has begun,
cancellation/timeout/disconnection can leave an uncertain outcome. Refresh
pending state and inspect recovery before retrying. A completed apply is
never converted into cancellation merely because a late cancel arrived.

Live SQL acceptance requires an authorized disposable SQL endpoint and is
not established by a file-database/native acceptance run. No ERP content is
needed or distributed by these tests. `.ibcmd-recovery` may contain private
recovery material: retain it locally and keep it outside published sources.


Opt-in native acceptance (fresh public `tiny-source`, no teardown): set
`IBCMD_TEST_BINARY` to the candidate, `IBCMD_TEST_NATIVE` to native ibcmd
8.5.1, and an absolute external `IBCMD_TEST_ROOT`, then run
`npm run test:native-apply`. It creates an `en_US_POSIX` disposable database,
imports an edited module through the real client/RPC, verifies that declining
apply changes no database bytes, applies through the client native route,
and compares the complete native export with the edited source. The
version manifest is checked separately for preserved names/IDs and the
changed module version. The script forces fixture retention for the server.

Local retained-fixture validation uses the focused `editor_server` integration
target, not the destructive root-unit test
`file_import::tests::owned_scratch_is_removed_on_success_error_and_unwind`.
That test's default cleanup assertions remain intact and apply only when
`IBCMD_TEST_KEEP_FIXTURES` is unset. The default production cleanup behavior is
unchanged; the explicit lab retention switch keeps import scratch evidence.
