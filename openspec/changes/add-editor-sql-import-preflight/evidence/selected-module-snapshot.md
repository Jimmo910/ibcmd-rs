# Snapshot-only existing SQL module compiler (#382)

The local implementation compiles one existing top-level CommonModule body from
an immutable Readset and supplied original-census source bytes. No SQL client,
fetch, filesystem read, diagnostic observer or publisher is used by production
compilation. The full readset digest binds the result to its original physical
snapshot; it is not SQL compare-and-swap or permission to write.

## Verification

Rust 1.95.0, jobs=1, external target/TMP, IBCMD_TEST_KEEP_FIXTURES=1:

- `cargo test --locked -p ibcmd-rs --lib server::sql_import::selected:: --no-default-features`:
  corrected run 10 passed, 0 failed, 0 ignored; compilation 41.56 s, tests 0.04 s.
- `cargo check --locked -p ibcmd-rs --lib --no-default-features`: exit 0, 15.48 s.
- Both builds retained the existing warning totals (40 lib-test / 97 production);
  no warning points into the new selected-module implementation.
- Independent source review cleared the complete delta and the later bounded
  parser correction. Formatting and diff whitespace checks passed.

The first run (5 passed / 4 failed) is retained. Its positive root fixture exposed
that the native family scanner rejects root's legitimate empty trailing cell.
The correction preserves the existing Brace parser and exact bytes, adding only
an allocation-free quote-aware nesting/cancellation preflight. The new explicit
trailing-cell, quoted-brace, depth and in-string cancellation test passes.

Tests cover SQL entries beyond declared count; unselected/service generations;
added/omitted/duplicate/reordered versions entries; UUID casing; wrong roles;
incomplete staging; root/descriptor identity and membership; name ambiguity;
unsupported container members; exact auxiliary info/header retention; a complete
staged neighbour; no-op output; budgets and cancellation. Fixtures use the existing
public tiny database and are retained unconditionally. Production compilation
accepts only the captured in-memory Readset, not its fixture's file adapter.

## Remaining acceptance

No real SQL compilation, transaction, CAS, COMMIT outcome, native apply or editor
SQL route was exercised or enabled. Document/form selection, source-bound plan
confirmation/TTL and the same-session publisher remain unfinished. The publisher
must preserve unselected physical staging, distinguish unknown COMMIT outcomes
and never retry blindly. This compiler does not turn arbitrary caller-made
Readsets into coherent or profile-admitted database captures.

The diagnostic observer and its unresolved baseline-registration gate are unchanged
and are not dependencies of this compiler. No baseline or whitelist was expanded.
No branch-wide quality, full SQL write acceptance or issue completion is claimed.

External receipts: `logs/issue-382-selected-module-receipt.json` and the three
`issue-382-selected-module-{first,corrected,check}.log` files. All build processes
exited and the local compiler slot is fully released.
