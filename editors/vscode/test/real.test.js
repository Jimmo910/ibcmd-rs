const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const { RpcClient } = require("../out/rpc");
const { editConfig, saveConfig } = require("../out/config");
const binary = process.env.IBCMD_TEST_BINARY;
test(
  "editor-created TOML is visible in real settings show and real stdio list",
  { skip: !binary },
  async () => {
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), "ibcmd-real-test-"));
    const file = path.join(dir, "ibcmd-rs.toml");
    const next = editConfig("# keep this comment\n", undefined, {
      server: "local-test",
      name: "Редактор",
      platform: "8.3.27",
    });
    await saveConfig(file, "", next);
    const env = { ...process.env, IBCMD_RS_CONFIG: file };
    delete env.IBCMD_DB_PSW;
    const output = execFileSync(
      binary,
      [
        "settings",
        "show",
        "--db-server",
        "local-test",
        "--db-name",
        "Редактор",
        "--json",
      ],
      { cwd: dir, env, encoding: "utf8" },
    );
    const report = JSON.parse(output);
    assert.equal(report.databases[0].name, "Редактор");
    assert.equal(report.databases[0].matches, true);
    const rpc = new RpcClient(binary, dir);
    try {
      const init = await rpc.request("initialize", {
        protocolVersion: "1.1",
        workspaceFolder: dir,
      });
      assert.equal(init.protocolVersion, "1.3");
      const listed = await rpc.request("infobases/list");
      assert.ok(listed.infobases.some((b) => b.id === "local-test/Редактор"));
      await assert.rejects(
        rpc.request("infobases/connect", {
          infobase: "local-test/Редактор",
          auth: "sql",
        }),
        (e) => e.code === -32602,
      );
    } finally {
      await rpc.dispose();
    }
  },
);
