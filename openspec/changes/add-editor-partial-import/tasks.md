- [x] Shared file database source with backward compatible settings and RPC schema
- [x] Fail-before-write regression tests
- [x] Locked effective-stage partial import and verification
- [x] Reference preflight, dependency confirmation, stale/cancel checks
- [ ] Editor actions and focused client tests
- [x] Native module + document form apply/export acceptance
- [x] Independent review, identity/secret checks, commits and approved tracking

This MCP branch includes the reviewed shared file-db backend and its
fail-before-write, guarded staging, reference, cancellation and recovery tests.
Synthetic native module/form apply/export evidence is recorded in
`evidence/native-8.5.1.md`; MCP module acceptance is recorded separately under
`../add-mcp-adapter/evidence/`. The editor partial-import UI/actions are owned by
PR #481 and deliberately excluded here, so that task remains unchecked.
Independent backend/MCP reviews and authorized PR tracking are recorded in
PR #483, which is Ready; neither issue completion nor merge/release is claimed.
Current-head CI and upstream maintainer workflow approval remain separate gates.
