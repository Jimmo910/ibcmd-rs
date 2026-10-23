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
  snapshot = async () => "",
  request = async () => undefined,
  source = "mssql",
} = {}) {
  const errors = [],
    infos = [];
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
  const bases = ["a", "b"].map((id) => ({
    id,
    name: id,
    source,
    platform: "8.3.27",
  }));
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
      if (method === "initialize") return {};
      if (method === "infobases/list") return { infobases: bases };
      if (method === "infobases/connect") return connect(params, signal);
      const result = await request(method, params, signal);
      return result ?? { conclusive: true, pending: false };
    }
  }
  const vscode = {
    StatusBarAlignment: { Left: 1 },
    ProgressLocation: { Notification: 1 },
    workspace: {
      isTrusted: true,
      workspaceFolders: [f],
      textDocuments: [],
      getConfiguration: () => ({
        get: (key, fallback) => values[key] ?? fallback,
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
      createOutputChannel: () => ({ appendLine() {}, show() {}, dispose() {} }),
      createStatusBarItem: () => ({ text: "", show() {}, dispose() {} }),
      showInputBox: input,
      showQuickPick: async (items) =>
        (await items).find((item) => item.base.id === picked),
      showErrorMessage(message) {
        errors.push(message);
      },
      showInformationMessage(message) {
        infos.push(message);
      },
      showWarningMessage: async (_, options, label) => label,
      withProgress: (_, task) => task({}, token),
    },
  };
  const module = { exports: {} };
  const extensionSource = fs.readFileSync(
    path.join(__dirname, "../out/extension.js"),
    "utf8",
  );
  vm.runInNewContext(extensionSource, {
    exports: module.exports,
    module,
    AbortController,
    require: (id) => {
      if (id === "vscode") return vscode;
      if (id === "./rpc")
        return { RpcClient, RpcError: class extends Error {} };
      if (id === "./config") return { snapshot };
      if (id === "./status") return require("../out/status");
      if (id === "./apply") return require("../out/apply");
      if (id === "./import") return require("../out/import");
      return require(id);
    },
  });
  const api = await module.exports.activate({
    subscriptions: [],
    secrets: {
      get: async () => stored,
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
    errors,
    infos,
    invoke: (name) => commands.get(name)(),
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

test("apply command refuses a connection changed during settings snapshot", async () => {
  let gate,
    entered,
    waitOnce = false;
  const methods = [];
  const h = await harness({
    stored: "cached",
    snapshot: async () => {
      if (waitOnce) {
        waitOnce = false;
        entered.resolve();
        await gate.promise;
      }
      return "";
    },
    request: async (method) => {
      methods.push(method);
      return undefined;
    },
  });
  await h.select("a");
  methods.length = 0;
  gate = deferred();
  entered = deferred();
  waitOnce = true;
  const applying = h.invoke("ibcmd.apply");
  await entered.promise;
  await h.select("b");
  gate.resolve();
  await applying;
  assert.ok(!methods.includes("config/apply"));
  assert.ok(!methods.includes("config/apply/prepare"));
});

test("pending snapshot failure after committed import preserves success", async () => {
  let published = false;
  const h = await harness({
    stored: "cached",
    source: "file-db",
    snapshot: async () => {
      if (published) throw new Error("unreadable settings");
      return "";
    },
    request: async (method) => {
      if (method === "objects/import/prepare")
        return {
          plan: "confirmed",
          preflight: {
            ready: true,
            files: ["CommonModules/Example/Ext/Module.bsl"],
            missingReferences: [],
          },
        };
      if (method === "objects/import") {
        published = true;
        return { report: {} };
      }
    },
  });
  await h.select("b");
  // runImport expects its own confirmation label.
  await h.invoke("ibcmd.importChanged");
  assert.equal(published, true);
  assert.deepEqual(h.errors, []);
  assert.ok(h.infos.some((message) => message.includes("CONFIGSAVE")));
});
