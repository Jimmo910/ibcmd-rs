#!/usr/bin/env node
// Synthetic status/lifecycle fixture only. It does not contact a database.
const fs = require("node:fs");
let input = Buffer.alloc(0);
function send(id, result, error) {
  const body = Buffer.from(
    JSON.stringify({ jsonrpc: "2.0", id, ...(error ? { error } : { result }) }),
  );
  process.stdout.write(
    Buffer.concat([
      Buffer.from(`Content-Length: ${body.length}\r\n\r\n`),
      body,
    ]),
  );
}
process.stdin.on("data", (chunk) => {
  input = Buffer.concat([input, chunk]);
  while (true) {
    const end = input.indexOf("\r\n\r\n");
    if (end < 0) return;
    const len = Number(
      /Content-Length: (\d+)/.exec(input.subarray(0, end).toString())[1],
    );
    if (input.length < end + 4 + len) return;
    const m = JSON.parse(input.subarray(end + 4, end + 4 + len));
    input = input.subarray(end + 4 + len);
    if (m.method === "exit") process.exit(0);
    if (m.id === undefined) continue;
    if (m.method === "initialize") {
      send(m.id, { protocolVersion: "1.1" });
      continue;
    }
    if (m.method === "shutdown") {
      send(m.id, null);
      continue;
    }
    if (m.method === "infobases/list") {
      const reply = () =>
        send(m.id, {
          infobases: ["a", "b"].map((name) => ({
            id: `fake/${name}`,
            name,
            server: "fake",
            platform: "8.3.27",
            source: "mssql",
            mask: false,
          })),
        });
      if (process.env.IBCMD_FAKE_MODE === "delayed-list") {
        const gate = process.env.IBCMD_FAKE_LIST_GATE;
        fs.writeFileSync(gate + ".started", String(process.pid));
        const wait = () =>
          fs.existsSync(gate) ? reply() : setTimeout(wait, 10);
        wait();
      } else reply();
      continue;
    }
    if (m.method === "infobases/connect") {
      const reply = () =>
        send(m.id, { infobase: m.params.infobase, platform: "8.5.1" });
      if (
        process.env.IBCMD_FAKE_MODE === "slow" &&
        m.params.infobase === "fake/a"
      )
        setTimeout(reply, 200);
      else reply();
      continue;
    }
    if (m.method === "config/pending") {
      if (process.env.IBCMD_FAKE_MODE === "crash") process.exit(9);
      if (process.env.IBCMD_FAKE_MODE === "error") {
        send(m.id, null, {
          code: -32014,
          message: "test secret must stay hidden",
        });
        continue;
      }
      send(m.id, {
        pending: process.env.IBCMD_FAKE_MODE === "pending",
        conclusive: process.env.IBCMD_FAKE_MODE !== "unknown",
      });
      continue;
    }
    send(m.id, null, { code: -32601, message: "unknown test method" });
  }
});
