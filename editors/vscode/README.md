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

Edit Connection supports SQL and read-only rows-directory entries. An omitted server
stays omitted so normal `db-server`/environment resolution still applies. Copying an
inherited rows-directory entry keeps its resolved source directory. File databases
and unknown source types are excluded from this editor; edit their settings using
the appropriate source-specific tooling instead.
