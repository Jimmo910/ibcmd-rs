# Issue 381 acceptance evidence — 2026-10-23

Dependency base: PR #484 head `8ddfa886` (includes connection UI #477), merged
with native export #485 head `218324cb` at `8f55a62e`. Donor branches/PRs were
not modified. The file database read backend inherited through #484 originates
in #481. No foreign #379 / PR #495 implementation is copied.

## Verified

- Rust 1.95.0, offline/locked, no default features, CARGO_BUILD_JOBS=2:
  `editor_export_plan`, `editor_file_db`, `editor_native_status`,
  `native_export_commands`, `native_export_status`: 39 passed; one optional
  private corpus test ignored. No failing tests. Fixtures retained.
- JavaScript/TypeScript client: 47 passed, zero skipped, with the actual built
  binary. Includes dirty object/inventory buffers, warning cancellation,
  explicit overwrite, untracked/ignored/staged paths, nested Git diff.relative,
  disk edits during confirmation, non-Git files, symlinks/hard links,
  case-insensitive dirty/Git paths, alias collision refusal, original backups
  and preservation of unrelated files.
- Actual installed VS Code host, unique external profile/workspace:
  dirty TextDocument refused; real stdio group export matched the native tiny
  module and ConfigDumpInfo bytes; database unchanged; commands registered.
  Final run exited 0 (`ISSUE381_HOST_PASS`).
- Fresh native `/opt/1cv8/8.5.1.1592/ibcmd` via Rosetta, explicit disposable
  `--db-path`, external `--data` and `--temp`: tiny selection and add/change/
  delete/rename cases matched native full-export byte trees and inventory.
  All five database files unchanged; unrelated source README preserved.
  Native full exports took 5.438–8.627 s; RPC plans 0.049–0.080 s on these tiny
  fixtures. These timings do not establish ERP performance.
- Rust formatting, Prettier, strict OpenSpec and unchanged physical-adapter
  policy baseline validated. Independent read-only review findings addressed:
  deleted descriptor removal; no whole-tree source scan; inventory hash/parser
  identity; staged diff.relative paths; concurrent save rechecks; non-Git
  warning; filesystem case aliases; hard links; hidden unrelated notes.

External retained logs: `logs/issue381-rust-focused.log`,
`logs/issue381-js-tests.log`, `logs/issue381-export-host-final.log`,
`logs/issue381-native.log`, `logs/issue381-policy.log` under the lab root.
Native aggregate manifest:
`fixtures/issue381-native-1792745781842521000/summary.json`.
Only repository-owned tiny fixtures were used. No private ERP XML/CF/data was
published or read by this acceptance run; original ERP is never a write target.

## Remaining acceptance

Exact-head remote CI is recorded in the Draft PR checks. ERP UH document latency
on an approved representative disposable fixture and live SQL selection/export
remain unverified. Case-only filesystem write/remove aliases are refused before
mutation and require resolution before retry. Interrupted publication is not
transactional; retained original bytes support recovery, and inventory is last.
The PR remains Draft and uses `Refs #381`; this evidence does not close the issue.
Project 9 writes were denied to the parent account; fields may remain stale and
were not retried through another route.
