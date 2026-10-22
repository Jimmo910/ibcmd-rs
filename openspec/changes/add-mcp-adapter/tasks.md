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
- [ ] Integrate #382 common settings/filedb session identity and staging API.
- [ ] Enforce agent-write per fixed database on every mutation and publication.
- [ ] Full status/compare including local-only paths through confined snapshots.
- [ ] Native acceptance: real MCP client edits disposable filedb module; external
      test orchestrator runs normal apply; native export matches source bytes.
- [ ] Parent review before PR creation. No release/merge/issue closure in this slice.
