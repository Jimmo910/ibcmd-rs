const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");
const {
  TreeModel,
  XmlSource,
  DatabaseSource,
  descriptor,
  relativePath,
  readResult,
} = require("../out/configuration-tree");
const signal = () => new AbortController().signal;
const xml = (kind, content) =>
  Buffer.from(
    `<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core"><${kind}>${content}</${kind}></MetaDataObject>`,
  );

test("сохранённая штатная выгрузка: состав дерева, модуль и поиск", async () => {
  const source = new XmlSource(
    path.resolve(__dirname, "../../../tests/fixtures/file-infobase/tiny-xml"),
  );
  const model = new TreeModel(source);
  try {
    const [root] = await model.children();
    const [general] = await model.children(root);
    assert.equal(general.label, "Общие");
    const groups = await model.children(general);
    const modules = groups.find((n) => n.kind === "CommonModule");
    const [module] = await model.children(modules);
    assert.equal(module.fullName, "CommonModule.ФайловаяБазаПроверка");
    const [file] = await model.children(module);
    assert.equal(
      file.path,
      "CommonModules/ФайловаяБазаПроверка/Ext/Module.bsl",
    );
    assert.equal(file.parent, module);
    const content = await source.read(file.path, signal());
    assert.deepEqual(
      content,
      await fs.readFile(path.join(source.directory, file.path)),
    );
    assert.equal((await model.search("файловая", signal())).length, 1);
    assert.equal((await model.children(modules))[0], module);
  } finally {
    model.dispose();
  }
});

test("вложенные реквизиты, табличные части, формы, синонимы и двоичные макеты", () => {
  const owner = {
    id: "Catalog.Товары",
    fullName: "Catalog.Товары",
    label: "Товары",
    kind: "Catalog",
    type: "object",
    path: "Catalogs/Товары.xml",
  };
  const result = descriptor(
    xml(
      "Catalog",
      `<Properties><Name>Товары</Name><Synonym><v8:item><v8:lang>ru</v8:lang><v8:content>Номенклатура &amp; услуги</v8:content></v8:item></Synonym></Properties><ChildObjects><Attribute><Properties><Name>Цена</Name></Properties></Attribute><TabularSection><Properties><Name>Строки</Name></Properties><ChildObjects><Attribute><Properties><Name>Количество</Name></Properties></Attribute></ChildObjects></TabularSection><Form>ФормаЭлемента</Form></ChildObjects>`,
    ),
    owner,
    [
      "Catalogs/Товары/Ext/ObjectModule.bsl",
      "Catalogs/Товары/Forms/ФормаЭлемента/Ext/Form.xml",
    ],
  );
  assert.equal(owner.synonym, "Номенклатура & услуги");
  assert.deepEqual(
    result.map((n) => n.kind),
    ["Attribute", "TabularSection", "Form", "file"],
  );
  assert.equal(result[0].children[0].type, "property");
  assert.equal(
    result[1].children[0].children[0].children[0].label,
    "Количество",
  );
  assert.equal(
    result[2].children[0].path,
    "Catalogs/Товары/Forms/ФормаЭлемента.xml",
  );
  assert.equal(result[3].path, "Catalogs/Товары/Ext/ObjectModule.bsl");
});

test("база читается по уровням; повторные и одновременные раскрытия используют кэш", async () => {
  const calls = [];
  const rpc = {
    request: async (method, params) => {
      calls.push({ method, params });
      return {
        nodes: params.node
          ? [
              {
                id: "Configuration/Catalogs",
                label: "Catalogs",
                kind: "Catalog",
                nodeType: "group",
              },
            ]
          : [
              {
                id: "Configuration",
                label: "База",
                kind: "Configuration",
                nodeType: "configuration",
                path: "Configuration.xml",
              },
            ],
      };
    },
  };
  const model = new TreeModel(new DatabaseSource(rpc, "/work", "demo"));
  const [[root], again] = await Promise.all([
    model.children(),
    model.children(),
  ]);
  assert.equal(root, again[0]);
  assert.equal(calls.length, 1);
  await model.children(root);
  await model.children(root);
  assert.equal(calls.length, 2);
  assert.ok(calls.every((c) => c.method === "tree/children"));
  model.dispose();
});

test("устаревший ответ после закрытия модели не возвращает узлы", async () => {
  let finish;
  const source = {
    children: () => new Promise((resolve) => (finish = resolve)),
  };
  const model = new TreeModel(source);
  const pending = model.children();
  model.dispose();
  finish([{ id: "old" }]);
  assert.deepEqual(await pending, []);
});

test("ошибка чтения допускает повтор, цикл и повторные узлы отклоняются", async () => {
  let count = 0;
  const model = new TreeModel({
    children: async () => {
      if (++count === 1) throw Error("сбой");
      return [{ id: "a" }, { id: "a" }];
    },
  });
  await assert.rejects(model.children());
  await assert.rejects(model.children(), /Повторяющийся/);
  assert.equal(count, 2);
  const cyclic = new TreeModel({ children: async () => [{ id: "a" }] });
  const [root] = await cyclic.children();
  await assert.rejects(cyclic.children(root), /Циклическое/);
  model.dispose();
  cyclic.dispose();
});

test("поиск по синониму и отмена поиска", async () => {
  const root = { id: "r", label: "Имя", type: "object" };
  const model = new TreeModel({
    children: async (node) => {
      if (!node) return [root];
      node.synonym = "Представление";
      return [];
    },
  });
  assert.deepEqual(await model.search("представ", signal()), [root]);
  const abort = new AbortController();
  abort.abort();
  await assert.rejects(model.search("a", abort.signal), /отменён/);
  model.dispose();
});

test("пути, повреждённый XML, DTD и неизвестные кодировки отклоняются", () => {
  for (const file of ["../x", "/root", "a\\b", "C:/x", "a//b", "a/../b"])
    assert.throws(() => relativePath(file));
  const owner = { kind: "Catalog", path: "Catalogs/Товар.xml" };
  assert.throws(() =>
    descriptor(Buffer.from("<!DOCTYPE x><MetaDataObject/>"), owner, []),
  );
  assert.throws(() =>
    descriptor(Buffer.from("<MetaDataObject><Catalog>"), owner, []),
  );
  assert.throws(() =>
    readResult(
      { path: "a", exists: true, encoding: "base64", content: "!!" },
      "a",
    ),
  );
  assert.deepEqual(
    readResult(
      { path: "a", exists: true, encoding: "base64", content: "AP8=" },
      "a",
    ),
    Buffer.from([0, 255]),
  );
});

test("XML-источник не читает ссылку за пределы каталога", async (t) => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "ibcmd-tree-"));
  try {
    await fs.mkdir(path.join(dir, "root"));
    await fs.mkdir(path.join(dir, "outside"));
    await fs.writeFile(
      path.join(dir, "outside", "secret.xml"),
      "закрытые данные",
    );
    await fs.symlink(
      path.join(dir, "outside"),
      path.join(dir, "root", "link"),
      "junction",
    );
    const source = new XmlSource(path.join(dir, "root"));
    await assert.rejects(
      source.read("link/secret.xml", signal()),
      /вне каталога/,
    );
  } finally {
    await fs.rm(dir, { recursive: true, force: true });
  }
});
