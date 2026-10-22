const { test } = require("node:test");
const assert = require("node:assert/strict");
const { runImport } = require("../out/import");
const request = {
  infobase: "lab",
  sourceDir: "/sources",
  files: ["Documents/Example/Ext/ObjectModule.bsl"],
};
const ready = {
  plan: "prepared",
  preflight: { ready: true, files: request.files, missingReferences: [] },
};
function harness(responses = [ready]) {
  const calls = [];
  const rpc = {
    request: async (method, params) => {
      calls.push({ method, params });
      return method === "objects/import/prepare"
        ? responses.shift()
        : { report: {} };
    },
  };
  return { calls, rpc };
}
test("only preflight runs when the final confirmation is declined", async () => {
  const { rpc, calls } = harness();
  assert.equal(
    await runImport(rpc, request, {
      current: async () => true,
      confirm: async () => false,
    }),
    undefined,
  );
  assert.deepEqual(
    calls.map((c) => c.method),
    ["objects/import/prepare"],
  );
});
test("connection change during confirmation refuses before publication", async () => {
  const { rpc, calls } = harness();
  let current = true;
  await assert.rejects(
    runImport(rpc, request, {
      current: async () => current,
      confirm: async () => {
        current = false;
        return true;
      },
    }),
    /Connection or settings changed/,
  );
  assert.equal(calls.length, 1);
});
test("cancellation during confirmation refuses before publication", async () => {
  const { rpc, calls } = harness();
  const abort = new AbortController();
  await assert.rejects(
    runImport(
      rpc,
      request,
      {
        current: async () => true,
        confirm: async () => {
          abort.abort();
          return true;
        },
      },
      abort.signal,
    ),
    /cancelled before publication/,
  );
  assert.equal(calls.length, 1);
});
test("dependency suggestion requires a separate explicit confirmation and new preflight", async () => {
  const missing = {
    plan: "unready",
    preflight: {
      ready: false,
      files: request.files,
      missingReferences: [
        {
          reference: "cfg:CatalogRef.Dependency",
          suggestedObject: "Catalog.Dependency",
        },
      ],
    },
  };
  const { rpc, calls } = harness([missing, ready]);
  let confirmations = 0;
  await runImport(rpc, request, {
    current: async () => true,
    confirm: async () => {
      confirmations++;
      return true;
    },
  });
  assert.equal(confirmations, 2);
  assert.deepEqual(
    calls.map((c) => c.method),
    ["objects/import/prepare", "objects/import/prepare", "objects/import"],
  );
  assert.deepEqual(calls[1].params.objects, [
    "Document.Example",
    "Catalog.Dependency",
  ]);
  assert.equal(calls[2].params.confirm, true);
});
test("declined dependency suggestion leaves the database unwritten", async () => {
  const { rpc, calls } = harness([
    {
      plan: "none",
      preflight: {
        ready: false,
        missingReferences: [
          {
            reference: "Catalog.Dependency",
            suggestedObject: "Catalog.Dependency",
          },
        ],
      },
    },
  ]);
  await runImport(rpc, request, {
    current: async () => true,
    confirm: async () => false,
  });
  assert.equal(calls.length, 1);
});
test("changed import preserves its selection and publishes the exact confirmed token", async () => {
  const { rpc, calls } = harness();
  await runImport(
    rpc,
    { infobase: "lab", sourceDir: "/sources", changed: true },
    { current: async () => true, confirm: async () => true },
  );
  assert.equal(calls[0].params.changed, true);
  assert.equal(calls[1].params.plan, "prepared");
});

test("changed dependency retry retains every original prepared file owner", async () => {
  const missing = {
    plan: "missing",
    preflight: {
      ready: false,
      files: request.files,
      missingReferences: [
        {
          reference: "Catalog.Dependency",
          suggestedObject: "Catalog.Dependency",
        },
      ],
    },
  };
  const { rpc, calls } = harness([missing, ready]);
  await runImport(
    rpc,
    { infobase: "lab", sourceDir: "/sources", changed: true },
    { current: async () => true, confirm: async () => true },
  );
  assert.deepEqual(calls[1].params.objects, [
    "Document.Example",
    "Catalog.Dependency",
  ]);
});
