// Execute the compiled extension state machine with deterministic synthetic dependencies.
// No SQL connection or real credentials are used by this harness.
const { test } = require("node:test");
const assert = require("node:assert/strict");
const vm = require("node:vm");
const fs = require("node:fs");
const path = require("node:path");
function deferred() {
  let resolve;
  const promise = new Promise((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
async function harness({
  stored,
  input = async () => "synthetic-new",
  connect = async () => ({}),
  store = async () => {},
  listed,
  text = "",
  initialize = async () => ({}),
} = {}) {
  const commands = new Map(),
    stores = [],
    clients = [];
  const f = {
    name: "fixture",
    uri: {
      scheme: "file",
      fsPath: "/synthetic",
      toString: () => "file:///synthetic",
    },
  };
  let picked = "a",
    cancel = () => {};
  const noEvent = () => ({ dispose() {} });
  const bases =
    listed ??
    ["a", "b"].map((id) => ({
      id,
      name: id,
      source: "mssql",
      platform: "8.3.27",
    }));
  const saved = [],
    errors = [],
    offered = [];
  const values = {
    executable: "synthetic",
    connections: ["a", "b"],
    connectionMetadata: {
      a: { auth: "sql", user: "fixture-user" },
      b: { auth: "windows" },
    },
  };
  const token = {
    isCancellationRequested: false,
    onCancellationRequested: (callback) => {
      cancel = () => {
        token.isCancellationRequested = true;
        callback();
      };
      return noEvent();
    },
  };
  class RpcClient {
    constructor() {
      this.disposed = false;
      clients.push(this);
    }
    async dispose() {
      this.disposed = true;
    }
    async request(method, params, signal) {
      if (method === "initialize") return initialize();
      if (method === "infobases/list") return { infobases: bases };
      if (method === "infobases/connect") return connect(params, signal);
      return { conclusive: true, pending: false };
    }
  }
  const vscode = {
    StatusBarAlignment: { Left: 1 },
    ConfigurationTarget: { WorkspaceFolder: 3 },
    ProgressLocation: { Notification: 1 },
    workspace: {
      isTrusted: true,
      textDocuments: [],
      workspaceFolders: [f],
      getConfiguration: () => ({
        get: (key, fallback) => values[key] ?? fallback,
        update: async (key, value) => {
          values[key] = value;
        },
      }),
      onDidChangeConfiguration: noEvent,
      onDidChangeWorkspaceFolders: noEvent,
      createFileSystemWatcher: () => ({
        onDidChange: noEvent,
        onDidCreate: noEvent,
        onDidDelete: noEvent,
        dispose() {},
      }),
    },
    commands: {
      registerCommand: (name, run) => {
        commands.set(name, run);
        return noEvent();
      },
    },
    window: {
      createStatusBarItem: () => ({ text: "", show() {}, dispose() {} }),
      showInputBox: input,
      showQuickPick: async (items) => {
        items = await items;
        offered.push(
          items.filter((item) => item.base).map((item) => item.base.id),
        );
        return (
          items.find((item) => item.base?.id === picked) ??
          items.find((item) => item.value === "windows")
        );
      },
      showErrorMessage(message) {
        errors.push(message);
      },
      showInformationMessage() {},
      withProgress: (_, task) => task({}, token),
    },
  };
  const module = { exports: {} };
  const source = fs.readFileSync(
    path.join(__dirname, "../out/extension.js"),
    "utf8",
  );
  vm.runInNewContext(source, {
    exports: module.exports,
    module,
    AbortController,
    require: (id) => {
      if (id === "vscode") return vscode;
      if (id === "./rpc")
        return { RpcClient, RpcError: class extends Error {} };
      if (id === "./config")
        return {
          ...require("../out/config"),
          snapshot: async () => text,
          saveConfig: async (_, expected, next) => {
            assert.equal(expected, text);
            saved.push(next);
          },
        };
      if (id === "./status") return require("../out/status");
      if (id === "./configuration-view")
        return {
          ConfigurationView: class {
            connect() {}
            disconnect() {}
          },
        };
      return require(id);
    },
  });
  const api = await module.exports.activate({
    subscriptions: [],
    secrets: {
      get: async () => stored,
      delete: async () => {},
      store: async (key, value) => {
        stores.push({ key, value });
        await store();
      },
    },
    workspaceState: { update: async () => {} },
  });
  return {
    stores,
    clients,
    saved,
    errors,
    offered,
    edit: (id) => {
      picked = id;
      return commands.get("ibcmd.edit")();
    },
    status: api.status,
    cancel: () => cancel(),
    select: (id = "a") => {
      picked = id;
      return commands.get("ibcmd.select")();
    },
  };
}
test("new password is persisted only after a successful current connect", async () => {
  const started = deferred(),
    finish = deferred();
  const h = await harness({
    connect: async () => {
      started.resolve();
      return finish.promise;
    },
  });
  const selecting = h.select();
  await started.promise;
  assert.equal(h.stores.length, 0);
  finish.resolve({});
  await selecting;
  assert.equal(h.stores.length, 1);
  assert.equal(h.stores[0].value, "synthetic-new");
});
test("failed login and cancelled prompt never store a new password", async () => {
  for (const options of [
    {
      connect: async () => {
        throw new Error("synthetic rejection");
      },
    },
    { input: async () => undefined },
  ]) {
    const h = await harness(options);
    await h.select();
    assert.equal(h.stores.length, 0);
  }
});
test("stale prompted password is not stored after a newer selection", async () => {
  const started = deferred(),
    finish = deferred();
  const h = await harness({
    input: () => {
      started.resolve();
      return finish.promise;
    },
  });
  const selecting = h.select();
  await started.promise;
  await h.select("b");
  finish.resolve("synthetic-stale");
  await selecting;
  assert.equal(h.stores.length, 0);
  assert.ok(h.status().includes(" b ·"));
});
test("stale or cancelled connect completion never commits a password", async () => {
  for (const stale of [true, false]) {
    const started = deferred(),
      finish = deferred();
    const h = await harness({
      connect: (params) => {
        if (params.infobase === "b") return {};
        started.resolve();
        return finish.promise;
      },
    });
    const selecting = h.select();
    await started.promise;
    if (stale) await h.select("b");
    else h.cancel();
    finish.resolve({});
    await selecting;
    assert.equal(h.stores.length, 0);
    if (!stale) {
      assert.ok(h.status().includes("disconnected"));
      assert.ok(h.clients.every((client) => client.disposed));
    }
  }
});
test("cancellation during non-cancellable secret storage still cleans up the session", async () => {
  const started = deferred(),
    finish = deferred();
  const h = await harness({
    store: () => {
      started.resolve();
      return finish.promise;
    },
  });
  const selecting = h.select();
  await started.promise;
  h.cancel();
  finish.resolve();
  await selecting;
  assert.ok(h.status().includes("disconnected"));
  assert.ok(h.clients.every((client) => client.disposed));
});
test("selection change during non-cancellable storage cannot restore stale status", async () => {
  const started = deferred(),
    finish = deferred();
  const h = await harness({
    store: () => {
      started.resolve();
      return finish.promise;
    },
  });
  const selecting = h.select();
  await started.promise;
  await h.select("b");
  finish.resolve();
  await selecting;
  assert.ok(h.status().includes(" b ·"));
});
test("a stored password is reused without prompting or rewriting on failure", async () => {
  const h = await harness({
    stored: "synthetic-existing",
    input: () => assert.fail("Unexpected password prompt"),
    connect: (params) => {
      assert.equal(params.password, "synthetic-existing");
      throw new Error("synthetic rejection");
    },
  });
  await h.select();
  assert.equal(h.stores.length, 0);
});

test("Edit preserves an omitted inherited server and excludes file databases", async () => {
  const text =
    'db-server="effective-host"\n[[database]]\nname="a"\nplatform="8.3.27"\n';
  const h = await harness({
    text,
    listed: [
      { id: "a", name: "a", platform: "8.3.27", source: "mssql" },
      { id: "file", name: "file", source: "file-db" },
      { id: "unknown", name: "unknown", source: "future-source" },
    ],
    input: async (options) => {
      if (options.prompt.startsWith("SQL Server")) {
        assert.equal(options.value, "");
        assert.equal(options.validateInput(""), undefined);
      }
      return options.value;
    },
  });
  await h.edit("a");
  assert.deepEqual(h.errors, []);
  assert.equal(h.saved.length, 1);
  assert.equal(
    require("../out/config").readConfig(h.saved[0])[0].server,
    undefined,
  );
  assert.deepEqual(Array.from(h.offered[0]), ["a"]);
});
test("Edit copies inherited offline rows with its source intact", async () => {
  const h = await harness({
    listed: [
      {
        id: "a",
        name: "a",
        platform: "8.3.27",
        source: "rows-dir",
        rowsDir: "/synthetic/original/rows",
      },
    ],
    input: async (options) => options.value,
  });
  await h.edit("a");
  assert.deepEqual(h.errors, []);
  assert.equal(h.saved.length, 1);
  assert.equal(
    require("../out/config").readConfig(h.saved[0])[0]["rows-dir"],
    "/synthetic/original/rows",
  );
});

test("Edit refuses inherited offline entries without a source directory", async () => {
  const h = await harness({
    listed: [{ id: "a", name: "a", platform: "8.3.27", source: "rows-dir" }],
    input: async (options) => options.value,
  });
  await h.edit("a");
  assert.equal(h.saved.length, 0);
  assert.equal(h.errors.length, 1);
});
