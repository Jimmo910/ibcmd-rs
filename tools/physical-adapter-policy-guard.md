# Physical-adapter policy guard

Run `pwsh -File tools/validate-physical-adapter-policy.ps1 -RepositoryRoot .`
and `pwsh -File tools/validate-physical-adapter-policy.ps1 -RepositoryRoot .
-SelfTest`.

The guarded production slice is `src/module_blob.rs` plus non-test Rust modules
under `src/mssql_dump/`, plus the non-test Rust modules under the `crates/`
directories that hold code moved out of them (`$GuardedCrateDirectories`):
`crates/ibcmd-1c-text/src/` (the 1C braced-text primitives and scalar fields,
and the source tree's file-name segment, formerly in `src/mssql_dump/mod.rs`,
`source_assets.rs` and `refs.rs`),
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
with its 22 record tags), `crates/ibcmd-metadata-blob/src/` (the packers of
a configuration's stored bodies from their XML sources, with the source
context they read, formerly `src/module_blob.rs`, which keeps the
command-line wrappers and the entry points that read the rest of the root
and stays in the slice; and the role-rights writer, formerly in
`src/compiler/bodies/`, and the metadata XML element tree, formerly
`src/metadata_model/xml.rs`, which were outside the slice and joined it with
their records), `crates/ibcmd-form-writer/src/` (the native form body writer,
formerly in `crates/ibcmd-metadata-blob/src/lib.rs` with the dynamic-list,
chart and planner writers it calls; its 2.21 route, formerly
`src/mssql_dump/form/{load_8_5_1,xml_2_21_writer,xml_2_21_order}.rs`; the
entry points that dispatch among them, formerly in `src/module_blob.rs`; and
the adopted-form compile, formerly `src/mssql/offline_compile/adopted_form.rs`,
which was outside the slice and joined it with its records),
`crates/ibcmd-module-blob/src/` (the module blob
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

`crates/ibcmd-metadata-source/src/` also belongs to the guarded slice. It
holds the `MetadataSource` trait through which the writers read the source
tree, which the packers' source context implements, and the source bytes and
memoised-answer types the trait returns, formerly in
`crates/ibcmd-metadata-blob/src/lib.rs`. The moved types hold no guarded
literal, so its one file joins the baseline with no occurrence and no record
of `ibcmd-metadata-blob` changes.

`crates/ibcmd-compiler-families/src/` also belongs to the guarded slice. It
contains the native family compilers, their bootstrap artifact/graph/identity
types, compiler axes and profile loader, formerly outside the slice under
`src/compiler/` and `src/profile_registry.rs`. All 798 existing production
fingerprint records join the slice unchanged; the move changes no fingerprint
or occurrence count. The earlier baseline records, including stale records,
remain untouched. The loader still embeds the canonical workspace `profiles/`
documents, with adjusted relative include paths and unchanged provenance names.
The subsequent mechanical clippy cleanup folds the identity collector's nested
configuration check into a short-circuiting let chain. Its one name-policy
fingerprint changes from
`06f20cdf6d98a912cc6698719fda5b234053dbc39bd3a4a6708ba1332a9289fb` to
`a1eb8c22ec2bfbf1d9159e97d2da8911a0a0d134b01e0e3abca8a2a4dc644539`,
with category and occurrence count (one) unchanged.

`crates/ibcmd-sql/src/` contains the database clients formerly in `src/sql/`,
which were outside the slice. Eight production files join it with their four
existing fingerprint records unchanged. The feature-gated live test module
stays excluded by its test-only declaration. All earlier records, including
the two stale records, remain unchanged.

`crates/ibcmd-restructure-engine/src/` and the existing portable model crate's
new `crates/ibcmd-restructure/src/descriptor/` directory also belong to the
slice. The engine holds the native restructuring planner, caches, database
execution and search-information records formerly under `src/restructure/`
and `src/mssql_config_apply/si.rs`, plus the apply model's text primitives.
The descriptor directory holds the shared object layouts and adoption
identity formerly under `src/metadata_model/objects*.rs` and
`src/mssql_dump/extension/mod.rs`. The portable model crate's earlier source
stays outside this slice, as the earlier MOXCEL decoder does. All 132 earlier
file records, including stale records, remain unchanged. Thirty-eight new
production file records join with their 300 existing fingerprint records.
The two public descriptor UUID constants retain their values and counts;
their fingerprints change only because `pub(crate)` becomes `pub`. The exact
before/after records are listed in the extraction's evidence. No baseline
refresh is used. Root CLI/settings/apply adapters remain in the root.

The guard tokenizes logical Rust statements outside comments and provably
test-only items, including multiline, raw-string, escaped-string and char forms.
It conservatively inventories immediate XML sink fragments and literal
concatenations.  It remains lexical inventory governance, not a full Rust parser
or general data-flow analysis, and cannot replace migration of XML policy into
schema-owned writers.

The standard common-command-group table in
`crates/ibcmd-1c-vocab/src/command_groups.rs`, formerly
`src/mssql_dump/mod.rs::COMMON_COMMAND_GROUPS`, is part of the same guarded
vocabulary directory. Its eleven existing UUID fingerprints and occurrence
counts move unchanged, record by record. The root retains its crate-visible
alias. All other existing file records, including stale records, remain
unchanged; neither the scope nor the baseline is regenerated.

`crates/ibcmd-dump-readers/src/` follows the same rule. The additional-index
reader, its owner model and the evidence-bound Configuration properties reader
moved from `src/mssql_dump/{additional_indexes,configuration_properties_evidence}.rs`.
Their two baseline file records move to the corresponding crate paths. All 11
existing additional-index fingerprint records keep their category, hash and
count; the Configuration reader and the new crate root have empty occurrence
lists. No fingerprint changes. Every other file record is preserved, including
the stale `config_dump_info.rs` and `incremental.rs` records. The source assets
module keeps the former owner-type path through an alias. Only existing text
and metadata-writer crates replace root aliases; no parsing policy changes.

Step 5f-1 of #559 (the form writer's preparation) adds three files to
guarded directories. `crates/ibcmd-1c-text/src/namespaces.rs` holds the 2.21
palette namespace rule, formerly in `src/mssql_dump/source_assets.rs`; its
four records (one `name-special-case`, three `xml-policy`) move unchanged.
`crates/ibcmd-1c-text/src/brace_list.rs`, the brace-list field splitter
formerly in `src/external/brace.rs`, and `crates/ibcmd-form-body/src/form_layout.rs`,
the stored form layout formerly in `src/platform/mod.rs`, were outside the
slice and join it with no occurrence. In `src/mssql_dump/form/xml_2_21_writer.rs`
one `xml-policy` fingerprint changes around an unchanged literal, because the
statement now names `ibcmd_form_body::parse_form_event_block` by its crate
path: `141bb3eb8dce1bd7cb7f0477f893d0e98d2e920fdd384fbb8d1c6311d39817a8` to
`3d272659525c543c2c5a548b0eec5ba3c5561a10d5023edb09bb1d094603420e`, count one.
Every other record, the two stale ones included, is unchanged.

Step 5f-2 of #559 moves the native form writer into `crates/ibcmd-form-writer/src/`.
The six whole files (`dynamic_list.rs`, `form_chart.rs`, `form_planner.rs`
from `crates/ibcmd-metadata-blob/src/`, and `form/load_8_5_1.rs`,
`form/xml_2_21_writer.rs`, `form/xml_2_21_order.rs` from
`src/mssql_dump/form/`) keep their records unchanged. The 2 088 records
(2 259 occurrences) of `crates/ibcmd-metadata-blob/src/lib.rs` go to the file
that holds each statement now: 1 605 / 1 742 to `crates/ibcmd-form-writer/src/lib.rs`
and 492 / 518 to `crates/ibcmd-metadata-blob/src/lib.rs`; eight records whose
statement is in both files split their count between them. Two fingerprints
change around an unchanged literal, count one each: `NATIVE_ZERO_UUID`
became `#[doc(hidden)] pub` for the packers (`uuid-literal`
`79321f56ff08c266e5bef695e597cbdb4703976af4543fa7f65d71ba384dd082` to
`739a0a806368259e55cc24fa2c21b12f3469d183731d2b01c8eeda2bdea78696`), and
the `b"BaseForm"` prefilter of `compile_native_form_body_through` moved from
`src/module_blob.rs` into the crate's `lib.rs`, its statement naming
`crate::adopted_form::` (`name-special-case`
`c75167dd7e5bd797140c48ef96cea1ccdada78ea1a954d18d29d6a2cbfd43e3e` to
`2047a68de8f2eab7285e22838c5231b344c7cd023239685ad0d51e32ae8de552`).
`src/module_blob.rs` stays in the slice with no occurrence.
`crates/ibcmd-form-writer/src/adopted_form.rs` was outside the slice and
joins it with its 32 records (33 occurrences). Every other record, the two
stale ones included, is unchanged.
