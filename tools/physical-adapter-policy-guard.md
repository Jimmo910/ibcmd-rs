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
command vocabularies, formerly in `src/mssql_dump/mod.rs`, `source_assets.rs`
and `command_interface.rs`; the role-right vocabulary, formerly in
`src/mssql_dump/role_rights.rs`; and the standard-picture table, formerly
`src/metadata_model/standard_pictures.rs`, which was outside the slice and
joined it with its 221 identifiers, and the compiler's built-in type table,
formerly in `src/metadata_model/types.rs`, which was outside the slice and
joined it with its 103 identifiers and one prefix check),
`crates/ibcmd-form-body/src/` (the form
body reader `form_body.rs`, formerly `src/mssql_dump/form_body.rs`; the
indexes, compatibility rules and timing report it is given, formerly in
`src/mssql_dump/mod.rs`, `refs.rs`, `dcs.rs`, `source_assets.rs`, `timing.rs`
and `form_ref_index.rs`; and the form body record, the event block and
adoption model, the 8.5 layout down-conversion with its group event table and
the data-composition normalizer it reads, formerly in `src/module_blob.rs`,
`src/mssql_dump/form_extension.rs`, `form/layout_8_5_1.rs`,
`form/xml_2_21_writer.rs` and `dcs.rs`; the record's reader with the physical
layout of its Form Attributes tail and the braced-record text primitives both
read, formerly in `src/module_blob.rs`; and the upgrade of an older platform's
root-`42` form body the reader applies first, formerly
`src/module_blob/form_root_42.rs`, which was outside the slice and joined it
with its 22 record tags), `crates/ibcmd-module-blob/src/` (the module blob
codec, the raw deflate helpers and digests, the raw-deflated packer and the
versions-blob patcher, formerly in `src/module_blob.rs`; the raw inflater,
formerly in `crates/ibcmd-1c-vocab/src/support.rs`; and the anyhow wrapper of
the V8 container, formerly `src/v8_container.rs`, which was outside the slice
and joined it with no occurrence) and
`crates/ibcmd-moxel/src/root_helpers/`.  Code that leaves the slice for a
crate takes the slice with it: its directory is listed
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
