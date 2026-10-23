# Issue 385 engineering evidence

Refs #385. Dependency: pinned PR #484 (`8ddfa886b8318d979dc322e5a3b98195e3dd5a57`),
which carries the connection/tree/status foundation from PR #477. No donor branch
was rewritten. Review the issue-385 delta after that pin; merge the dependency first.
Package tooling/lockfile/workflows are owned here; #381/#384 provide feature command
and menu additions independently. No Rust production code changes in this delta.

## Local acceptance — 2026-10-23

- Locked npm install, esbuild bundle, 33 client unit/real CLI tests, 2 executable
  resolution tests and 2 archive tests passed. Format and diff whitespace checks passed.
- Full VS Code extension-host suite passed: connection lifecycle, real committed-CF
  rows inventory/export and local edits, synthetic lifecycle/six-state failures,
  Explorer/filter, cancellation/stale responses and multi-root removal. Synthetic
  controls are not evidence of a live SQL login.
- Two complete Mac VSIX packages were byte-identical at SOURCE_DATE_EPOCH 1792729235:
  SHA-256 `856816fba05784a78b27d079cff260ae0669bcc1f0b0cbc589b64e8db7831c29`.
  Subsequent metadata-only npm script portability edits do not alter the runtime
  bundle; exact-head Windows CI repeats full packaging comparison.
- Actual installed VSIX acceptance passed in VS Code 1.137.0 and Cursor 1.105.1,
  using separate clean profiles. Tests assert installed extension location, invoke
  the bundled Mac executable, match the four fixture object identities and visible
  tree, then repeat after a machine `ibcmd-rs.path` override. Both hosts exited 0.
- Fixture objects: Configuration, Language.Русский, CommonForm.ФормаСправа,
  CommonForm.ФормаСлева. Unknown status without baseline is expected. No SQL/native
  version equivalence is claimed by this installation test.
- Independent review identified Windows `.cmd` launch, stale bundle and SVG package
  defects; all addressed. Recheck prompted portable Python selection and explicit
  CI Python provisioning, also addressed.

Local retained evidence (all under `/Volumes/Work/Dev/ibcmd-rs-lab`):

- `logs/issue-385-full-host.log`
- `logs/issue-385-final-vscode.log`
- `logs/issue-385-final-cursor.log`
- `tmp/issue-385-final-vscode/installed-u72Sm0/acceptance.json`
- `tmp/issue-385-final-cursor/installed-vUI86Q/acceptance.json`
- `build/issue-385/final-a.vsix` and `final-b.vsix`

The Mac package uses retained `build/issue-380/debug/ibcmd-rs`, built from the pinned
Rust dependency. This is installation evidence, not a production release binary.
CI builds its own Windows release binary with Rust 1.95.0; the release workflow
packages the matching audited Windows release binary. Local new fixture/staging
outputs are retained; no permanent deletion or original ERP access occurred.

## Remaining owner/external acceptance

No release was dispatched or created, and Marketplace/Open VSX publication and
publisher credentials were not configured. The owner must perform publication and
release gates. No VSIX was installed from an already published release; the test
installs the locally built candidate. Windows acceptance is provided by the new
CI job, not by this Mac run. Screenshot capture was unavailable because Computer
Use permissions were not granted; README uses an explicitly labeled illustration.
GitHub Project 9 updates are unavailable to this account, per coordinator evidence;
no denied board write was retried. Keep the full issue open and this partial-scope
PR Draft until the owner's remaining acceptance is satisfied.
