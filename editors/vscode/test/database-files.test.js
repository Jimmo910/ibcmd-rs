const { test } = require("node:test");
const assert = require("node:assert/strict");
const vm = require("node:vm");
const fs = require("node:fs");
const path = require("node:path");
const core = require("../out/configuration-tree");
function load() {
  class FileSystemError extends Error {}
  for (const code of [
    "FileNotFound",
    "Unavailable",
    "FileNotADirectory",
    "NoPermissions",
  ])
    FileSystemError[code] = () =>
      Object.assign(new FileSystemError(code), { code });
  class Uri {
    constructor(value) {
      Object.assign(this, value);
    }
    toString() {
      return JSON.stringify({
        scheme: this.scheme,
        path: this.path,
        query: this.query,
      });
    }
    static from(value) {
      return new Uri(value);
    }
    static parse(value) {
      return new Uri(JSON.parse(value));
    }
  }
  const vscode = {
    Uri,
    FileSystemError,
    FileChangeType: { Deleted: 3 },
    FileType: { File: 1 },
    FilePermission: { Readonly: 1 },
    Disposable: class {
      dispose() {}
    },
    EventEmitter: class {
      event = () => ({ dispose() {} });
      fire() {}
      dispose() {}
    },
    workspace: { isTrusted: true },
  };
  const module = { exports: {} };
  vm.runInNewContext(
    fs.readFileSync(path.join(__dirname, "../out/database-files.js"), "utf8"),
    {
      module,
      exports: module.exports,
      AbortController,
      Uint8Array,
      Buffer,
      require: (id) =>
        id === "vscode"
          ? vscode
          : id === "./configuration-tree"
            ? core
            : require(id),
    },
  );
  return { ...module.exports, vscode };
}

test("виртуальный файл сохраняет точные байты и отказывает во всех изменениях", async () => {
  const { DatabaseFiles, vscode } = load();
  const files = new DatabaseFiles();
  const bytes = Buffer.from([239, 187, 191, 0, 255, 13, 10]);
  const source = new core.DatabaseSource(
    {
      request: async () => ({
        path: "Templates/Макет/Ext/Template.bin",
        exists: true,
        encoding: "base64",
        content: bytes.toString("base64"),
      }),
    },
    "/work",
    "База",
  );
  const uri = files.uri(source, "Templates/Макет/Ext/Template.bin");
  assert.deepEqual(await files.readFile(uri), bytes);
  const stat = await files.stat(uri);
  assert.equal(stat.size, bytes.length);
  assert.equal(stat.permissions, vscode.FilePermission.Readonly);
  for (const method of ["writeFile", "delete", "rename", "createDirectory"])
    assert.throws(() => files[method](uri), { code: "NoPermissions" });
  files.dispose();
});

test("неизвестные, отсутствующие и устаревшие файлы не подменяются другой базой", async () => {
  const { DatabaseFiles, vscode } = load();
  const files = new DatabaseFiles();
  const source = { read: async () => undefined };
  const uri = files.uri(source, "Configuration.xml");
  await assert.rejects(files.readFile(uri), { code: "FileNotFound" });
  const empty = files.uri(source, "Configuration.xml", true);
  assert.equal((await files.readFile(empty)).byteLength, 0);
  await assert.rejects(
    files.readFile(vscode.Uri.from({ scheme: "ibcmd-rs", path: "/unknown" })),
    { code: "FileNotFound" },
  );
  files.invalidate();
  const next = files.uri(
    { read: async () => Buffer.from("новая база") },
    "Configuration.xml",
  );
  assert.notEqual(next.toString(), uri.toString());
  await assert.rejects(files.readFile(uri), { code: "FileNotFound" });
  assert.equal((await files.readFile(next)).toString(), "новая база");
  files.dispose();
});

test("смена источника отменяет чтение; запросы одного файла объединяются", async () => {
  const { DatabaseFiles } = load();
  const files = new DatabaseFiles();
  let finish,
    signal,
    calls = 0;
  const source = {
    read: async (_, s) => {
      calls++;
      signal = s;
      return new Promise((resolve) => (finish = resolve));
    },
  };
  const uri = files.uri(source, "Ext/Module.bsl");
  const a = files.readFile(uri),
    b = files.readFile(uri);
  assert.equal(calls, 1);
  files.invalidate();
  assert.equal(signal.aborted, true);
  finish(Buffer.from("прежние данные"));
  await assert.rejects(a, { code: "Unavailable" });
  await assert.rejects(b, { code: "Unavailable" });
  files.dispose();
});

test("ошибки сервера не раскрывают его текст; недоверенная область не читается", async () => {
  const { DatabaseFiles, vscode } = load();
  const files = new DatabaseFiles();
  const uri = files.uri(
    {
      read: async () => {
        throw Error("секретные данные сервера");
      },
    },
    "Configuration.xml",
  );
  await assert.rejects(
    files.readFile(uri),
    (e) => e.code === "Unavailable" && !e.message.includes("секретные"),
  );
  vscode.workspace.isTrusted = false;
  await assert.rejects(files.readFile(uri), { code: "NoPermissions" });
  files.dispose();
});

test("сравнение объединяет файлы групп и различает отсутствующие стороны", () => {
  const { differences } = load();
  const row = {
    object: "Catalog.Товары",
    changed: ["Catalogs/Товары.xml"],
    baseOnly: ["Catalogs/Товары/Ext/Module.bsl"],
    localOnly: ["Catalogs/Товары/Ext/Old.bsl"],
  };
  const result = differences({ objects: [row, row] });
  assert.equal(result.length, 3);
  assert.equal(
    result.find((x) => x.path.endsWith("Old.bsl")).state,
    "localOnly",
  );
  assert.throws(() =>
    differences({ objects: [{ ...row, localOnly: ["../secret"] }] }),
  );
  assert.throws(() =>
    differences({ objects: [{ ...row, localOnly: row.changed }] }),
  );
  assert.throws(() => differences({ objects: [{ object: "Catalog.X" }] }));
});
