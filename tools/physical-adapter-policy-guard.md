# Physical-adapter policy guard

Run `pwsh -File tools/validate-physical-adapter-policy.ps1 -RepositoryRoot .`
and `pwsh -File tools/validate-physical-adapter-policy.ps1 -RepositoryRoot .
-SelfTest`.

The guarded production slice is `src/module_blob.rs` plus non-test Rust modules
under `src/mssql_dump/`, plus the files under `crates/` that hold code moved out
of them (`$GuardedCrateFiles`: `crates/ibcmd-moxel/src/root_helpers.rs`).  Code
that leaves the slice for a crate takes the slice with it: its file is listed
there and its baseline entries move to the new path in the same change, never
dropped.  The MOXCEL decoder itself (`crates/ibcmd-moxel`, formerly
`src/mssql_dump/moxel.rs` and `mxl_ir.rs`) was outside the slice and stays
outside it; its root entry points in `src/mssql_dump/moxel_entry.rs` are guarded.
A module the compiler reads only for tests is not production and is left out
with every file below its directory: the guard finds it from the declaration
`#[cfg(test)] mod name;` (also `pub`, or with other attributes) in a scoped
file, so `tests.rs`, `tests/`, `metadata_order_tests.rs` and
`revision_mix_tests.rs` need no listing and the next `*_tests.rs` file needs
none either.  A module with a
`#[path = "..."]` attribute is not resolved and stays guarded.  Inline items are
excluded only when their `cfg(...)` predicate requires `test=true`; ambiguous
or partly-production predicates remain guarded.  Schema-owned accessor calls without local literals
add no inventory.  This intentionally does not cover `src/source_oracle.rs`,
CLI/parity documentation, or MXL/MOXL production.

The baseline holds hashes and counts only.  A new scoped file, occurrence, or
increased count fails; removing entries from source passes.  To intentionally
add a reviewed compatibility policy, run the validator with `-WriteBaseline`,
inspect the hash-only diff alongside the source change, and commit both.  Do not
use a baseline refresh to conceal unrelated literals.  Fingerprint inputs
canonicalize CRLF and bare CR to LF, so platform line endings do not churn the
baseline.

The guard tokenizes logical Rust statements outside comments and provably
test-only items, including multiline, raw-string, escaped-string and char forms.
It conservatively inventories immediate XML sink fragments and literal
concatenations.  It remains lexical inventory governance, not a full Rust parser
or general data-flow analysis, and cannot replace migration of XML policy into
schema-owned writers.
