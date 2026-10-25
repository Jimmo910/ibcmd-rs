# Physical-adapter policy guard

Run `pwsh -File tools/validate-physical-adapter-policy.ps1 -RepositoryRoot .`
and `pwsh -File tools/validate-physical-adapter-policy.ps1 -RepositoryRoot .
-SelfTest`.

The guarded production slice is `src/module_blob.rs` plus non-test Rust modules
under `src/mssql_dump/`, plus the non-test Rust modules under the `crates/`
directories that hold code moved out of them (`$GuardedCrateDirectories`):
`crates/ibcmd-1c-text/src/` (the 1C braced-text primitives and scalar fields,
formerly in `src/mssql_dump/mod.rs`, `source_assets.rs` and `refs.rs`),
`crates/ibcmd-1c-vocab/src/` (the platform-type, standard-attribute, style and
command vocabularies, formerly in `src/mssql_dump/mod.rs` and
`source_assets.rs`) and `crates/ibcmd-moxel/src/root_helpers/`.  Code that
leaves the slice for a crate takes the slice with it: its directory is listed
there and its baseline entries move to the new path in the same change, never
dropped.  Every `*.rs` file below a listed directory is scanned, so a file added
there is in the slice at once and fails as unbaselined until its record is
reviewed, a file deleted from one fails while its record is in the baseline,
and a listed directory that is missing fails the guard.  A crate made
only of moved code is listed by its `src/`.  The MOXCEL decoder itself
(`crates/ibcmd-moxel`, formerly `src/mssql_dump/moxel.rs` and `mxl_ir.rs`) was
outside the slice and stays outside it, so only the module directory that holds
what it took from the slice is listed; its root entry points in
`src/mssql_dump/moxel_entry.rs` are guarded.
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
baseline.  A fingerprint hashes the whole logical statement around the literal,
and the scanner's paren depth runs across the file, so code that moves can
change fingerprints without changing its literals: a `const` made `pub`, or a
`match` whose arms the scanner splits differently in the new file.  A move
edits the baseline record by record, lists every changed fingerprint, and keeps
the records of files it does not touch, stale ones included.

The guard tokenizes logical Rust statements outside comments and provably
test-only items, including multiline, raw-string, escaped-string and char forms.
It conservatively inventories immediate XML sink fragments and literal
concatenations.  It remains lexical inventory governance, not a full Rust parser
or general data-flow analysis, and cannot replace migration of XML policy into
schema-owned writers.
