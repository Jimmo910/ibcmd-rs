const vscode = require("vscode");
const fs = require("node:fs/promises");
const path = require("node:path");
const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
exports.run = async () => {
  const ext = vscode.extensions.getExtension("ibcmd-rs.ibcmd-rs-connections");
  assert.ok(ext, "VSIX extension must be installed");
  assert.ok(
    ext.extensionPath.startsWith(process.env.IBCMD_INSTALLED_ROOT),
    ext.extensionPath,
  );
  const binary = path.join(
    ext.extensionPath,
    "bin",
    process.platform === "win32" ? "ibcmd-rs.exe" : "ibcmd-rs",
  );
  const f = vscode.workspace.workspaceFolders[0],
    source = path.join(f.uri.fsPath, "source");
  await fs.mkdir(source);
  const cli = (...args) =>
    JSON.parse(
      execFileSync(binary, args, { cwd: f.uri.fsPath, encoding: "utf8" }),
    );
  const rows = process.env.IBCMD_TEST_ROWS;
  const inventory = cli(
    "objects",
    "status",
    "--platform",
    "8.3.27",
    "--rows-dir",
    rows,
    "--source-dir",
    source,
  ).objects;
  assert.ok(inventory.length > 0);
  const args = [
    "objects",
    "export",
    "--platform",
    "8.3.27",
    "--rows-dir",
    rows,
    "--target-dir",
    source,
  ];
  for (const item of inventory) args.push("--object", item.object);
  cli(...args);
  await fs.writeFile(
    path.join(f.uri.fsPath, "ibcmd-rs.toml"),
    '[[database]]\nserver="lab"\nname="offline"\nplatform="8.3.27"\nrows-dir=' +
      JSON.stringify(rows) +
      "\n",
  );
  const cfg = vscode.workspace.getConfiguration("ibcmd", f.uri);
  await cfg.update(
    "connections",
    ["lab/offline"],
    vscode.ConfigurationTarget.WorkspaceFolder,
  );
  await cfg.update(
    "connectionMetadata",
    { "lab/offline": { auth: "windows" } },
    vscode.ConfigurationTarget.WorkspaceFolder,
  );
  await cfg.update(
    "sourceDirectory",
    source,
    vscode.ConfigurationTarget.WorkspaceFolder,
  );
  await cfg.update(
    "statusComparison",
    "bytes",
    vscode.ConfigurationTarget.WorkspaceFolder,
  );
  const api = await ext.activate();
  const quick = vscode.window.showQuickPick,
    error = vscode.window.showErrorMessage;
  const errors = [];
  vscode.window.showQuickPick = async (items) => (await items)[0];
  vscode.window.showErrorMessage = async (msg) => {
    errors.push(msg);
  };
  try {
    await vscode.commands.executeCommand("ibcmd.select");
    await vscode.commands.executeCommand("ibcmd.objects.refresh");
    assert.deepEqual(errors, []);
    assert.deepEqual(
      api
        .objects()
        .map((r) => r.object)
        .sort(),
      inventory.map((r) => r.object).sort(),
    );
    assert.equal((await api.visibleObjects()).length, inventory.length);
    const override = vscode.workspace.getConfiguration("ibcmd-rs");
    await override.update("path", binary, vscode.ConfigurationTarget.Global);
    await vscode.commands.executeCommand("ibcmd.select");
    await vscode.commands.executeCommand("ibcmd.objects.refresh");
    assert.deepEqual(
      api
        .objects()
        .map((r) => r.object)
        .sort(),
      inventory.map((r) => r.object).sort(),
    );
    assert.deepEqual(errors, []);
    const evidence = {
      editor: vscode.env.appName,
      version: vscode.version,
      installed: ext.extensionPath,
      binary,
      objects: api.objects(),
      override: true,
    };
    await fs.writeFile(
      path.join(process.env.IBCMD_ACCEPTANCE_RUN, "acceptance.json"),
      JSON.stringify(evidence, null, 2),
    );
    console.log(JSON.stringify(evidence));
  } finally {
    vscode.window.showQuickPick = quick;
    vscode.window.showErrorMessage = error;
  }
};
