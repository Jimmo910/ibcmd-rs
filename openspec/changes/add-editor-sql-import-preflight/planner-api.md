# Backend-neutral planner contract

The mechanical dependency is crate::import_plan::prepare(effective_rows,
source, selection, platform), under the existing export-engine lock.

- effective_rows is an owned BTreeMap<String, Vec<u8>> of immutable, already
  validated effective CONFIG + CONFIGSAVE packed entries.
- source is the original HeldSourceRoot, not a fresh authorization by pathname.
- Selection and MissingReference remain reachable at file_import's public paths
  through reexports; their fields/serialization are unchanged.
- Prepared owns source, changed files, reference diagnostics and either fully
  compiled/mandatory-verified packed entries or no entries for unresolved refs.
- The caller retains database identity/schema/readset/CAS/locks, source final
  revalidation after adapter packing, cancellation and publication outcomes.
- The shared planner has no session, database pathname or publication method.
- FileDb still assembles its effective rows and materializes its original row
  format/deletion list outside this planner; all final guards and Plan methods
  remain in file_import unchanged. SQL needs its own immutable snapshot and
  exact unselected-stage preservation, not the FileDb whole-stage conversion.

Preservation/refusal contracts precede the extraction: one test seeds a valid
previous Language change in effective rows, prepares a different module change,
and requires the previous language payload to remain byte-identical. Another
removes an actual existing baseline module (retaining its artifact outside the source) and adds an unsupported new body; both must refuse. This pure planner test is not adapter/no-publication evidence.
Existing FileDb partial/reference/stale/cancellation tests remain unchanged.
Tests are source-only and unrun until a Cargo slot is granted. A current-master
cleanup unit has removal assertions while its runtime scratch honors KEEP1;
any failure/pre-existing label requires a base/head reproduction, not a waiver.
