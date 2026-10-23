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

## Dependency audit follow-up

The initial vsce 3.6.2 pin reported six high-severity transitive advisories. It was
replaced by exact `@vscode/vsce` 4.0.0 (Node >=22, matching CI). `npm audit` now reports
zero advisories at every severity. No scanner or baseline was disabled. Bundle,
format, executable/archive tests and actual packaging pass with the patched tool.
The retained patched packaging audit is `logs/issue-385-npm-audit-patched.json`.
Two patched complete VSIX files are byte-identical with SHA-256
`6863b55a6048b34eafd46b8ce9cdc0de6e9bbef75bb74333943dfabdd67c6c49`;
see `build/issue-385/patched-vsce-{a,b}.vsix`. This supersedes the earlier package
hash above. Installed checks also passed again with this patched tool in
`tmp/issue-385-vsce4-vscode/installed-9RWXtY/acceptance.json` and
`tmp/issue-385-vsce4-cursor/installed-xknz3f/acceptance.json`; logs are
`logs/issue-385-vsce4-{vscode,cursor}.log`. The normalized epoch was the timestamp of the preceding engineering
commit (0bf5b256); changing SOURCE_DATE_EPOCH intentionally changes ZIP bytes.

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

## Windows checkout formatting regression

Current-head Windows job [120334143270](https://github.com/Jimmo910/ibcmd-rs/actions/runs/39929553539/job/120334143270)
failed `npm run format:check` at a3488081: `scripts/build.cjs` and
`scripts/python.cjs` were converted to CRLF because the editor attributes covered
only .ts/.js/.json. The same failure was reproduced locally with
`git -c core.autocrlf=true checkout-index` in a retained external snapshot; both
scripts contained CRLF and Prettier exited 1.

Adding `.cjs text eol=lf` makes the identical forced checkout pass Prettier. A new
package regression performs that checkout and asserts LF without CRLF in both
scripts. All three Python archive/checkout tests, two Node executable tests,
`format:check` and whitespace checks pass. Independent read-only review found no
blocker. Formatter settings and checks are unchanged. Before/after snapshots are
retained under `tmp/issue-385-crlf-{before,after}-*`; the failed Windows job log is
`logs/issue-385-windows-a3488081.log`. Windows exact-head CI remains the final proof.
