# Implementation and remaining acceptance

- [x] Verify official protocol revision and current published Rust SDK.
- [x] Separate MCP stdio command with six described tools and schemas.
- [x] Fixed operator policy and deny-by-default import, no model target override.
- [x] Confined bounded offline snapshots through the existing common engine.
- [x] Cancellation bridge, bounded admission and redacted tool errors.
- [x] Independent security/code review and fixes of discovered confinement issues.
- [x] Actual rmcp client: modern discovery and legacy initialize/cancellation; offline
      export/read parity with cf export; policy/target/traversal/symlink failures.
      `cargo test --test mcp_adapter --test editor_server`: 2 MCP + 6 editor tests pass.
      Log: external lab logs/issue-386-regression.log.
- [x] Integrate READ dependency 644684f2 as ea8097f5; common-file [mcp] supported.
      5 actual1CD backend + 6 editor + 2 MCP tests pass after integration.
      Log: external lab logs/issue-386-read-integration-final.log.
- [ ] Integrate #382 descriptor filedb identity and staging API.
- [ ] Enforce agent-write per fixed database on every mutation and publication.
- [ ] Full status/compare including local-only paths through confined snapshots.
- [ ] Native acceptance: real MCP client edits disposable filedb module; external
      test orchestrator runs normal apply; native export matches source bytes.
- [ ] Parent review before PR creation. No release/merge/issue closure in this slice.
