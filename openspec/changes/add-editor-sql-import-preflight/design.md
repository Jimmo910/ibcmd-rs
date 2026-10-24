# SQL editor preflight design

## Acceptance slice and ownership

The primary files are src/server/sql_import.rs, tests/editor_sql_import.rs and
this change. Server module routing/token-store wiring is coordinated separately
and belongs to the same owner. file_import.rs, ibcmd-filedb, form_body and moxel
are unchanged. The base is verified master 6ca6ade04ceee151f056e29acb5cd6deaaa9fbc4.

## Immutable plan and compiler boundary

Selection admits exactly one of files, objects or changed. Changed mode requires
matching synchronized ConfigDumpInfo ownership/versions. Source bytes belong to
a bounded HeldSourceRoot census; selected inputs cannot be reopened as new
authority after preparation. Names, symlinks, aliases, paths and resource limits
are checked before SQL mutation. Target metadata comes from the consumed SQL
connection, including stable database identity rather than only display name.

The plan owns the verified proposed row payload, exact baseline readset and
original staging, selected row keys, source witness, connection binding and
reference diagnostics. No client-supplied SQL or payload is authoritative.
Target/source/identity changes invalidate it. Missing references create an
unpublishable plan; accepting suggested dependencies constructs a new plan.
New/deleted/structural or dynamic-generation changes outside the established
compiler cohort are refused; a form is accepted only if the existing compiler
and staged-state verification can represent it without renderer changes.

## Transaction boundary

SqlClient requests normally open separate sessions. A query followed by execute
cannot be CAS. Publication therefore uses one run_script session and one
transaction, with fail-closed identity and complete readset checks under retained
range/table locks before target DML. Final staged rows are the baseline stage
with only selected keys replaced. Existing unselected staging is preserved;
unsupported/deletion/unrecognized stages refuse before any target DML.

The old replace_config_save stage is not a publication adapter. Its existing
compiler/verifier may be reused only through an immutable, no-target-write
preparation boundary. Ambient script/guard bypass switches cannot disable editor
verification. Generated legacy scripts are not blindly wrapped in another
transaction: batch/session and nested transaction semantics need explicit proof.

Source and cancellation checks run immediately before dispatch. A transaction
error rolls back. Cancellation or disconnect after commit is not a false
no-write result: publication outcome must be reconciled or reported uncertain.
No dynamic activation or platform subprocess is part of production preparation.

## Clean-room validation and remaining evidence

Synthetic rows and source fixtures exercise declined consent, unresolved
references, changed source/connection/target, existing unselected staging,
unsupported inputs, limits, cancellation and same-session failure ordering.
Recording/fault backends may prove orchestration, not real SQL atomicity.
Actual disposable SQL acceptance and native apply/export are run only through
the single authorized VM operator in an allocated slot. Main module plus form
must reproduce expected source after native apply/export, with an unselected
object and pre-existing staging preserved. No SQL capability is advertised ready
before this gate. No Cargo/native/VM run belongs to initial design work.

## Initial source-admission implementation

The isolated SourceAdmission helper performs no SQL and does not remove the
existing RPC refusal. Defaults are 10,000 files, 16 MiB per file, 128 MiB total,
20,000 filesystem entries and depth 64. Limits are operator-side; request data
cannot raise them. It reuses HeldSourceRoot and the existing files selector,
retains immutable selected bytes and validates changed-mode CDI syntax. Actual
CDI/target ownership comparison, reference checks, compiler verification,
identity/readset capture and transaction execution remain separate pending work.
Source admission tests passed on e6150297 (6 tests); the corrected c9a229fa
checkpoint differs only by removal of an unused neutral-planner import and passed
a library check. These results do not validate the later physical readset model.

## Physical readset model (synthetic verification only)

The SQL-owned readset module has no SQL I/O or mutation API. Its builder admits
bounded physical CONFIG/CONFIGSAVE parts with actual identity, a schema digest,
complete canonical opaque headers and immutable blob bytes. Duplicate parts,
gaps, inconsistent/truncated sizes and resource/cancellation failures invalidate
the model; an admission failure poisons the entire builder. Local plan binding
covers every identity/schema/key/part/header/size/blob, including unrelated rows.
A pure selected-stage merge retains every unselected part and header and refuses
new stored names. This is row arithmetic only: Rust construction/digest comparison
does not prove coherent SQL acquisition, compiler authority or atomic CAS. Those
remain required adapter/transaction gates and no RPC SQL route is enabled.

Defaults for this model are 100,000 physical parts, 16 MiB per part, 64 MiB total
payload, 8 MiB metadata and 128 UTF-16 name units. All checks remain operator-side.
The source admission/readset integration passed all 14 tests on the c9a229fa
checkpoint plus the readset source hashes recorded in the external verification
receipt. The subsequent library check exited 0; its 98 root warnings are outside
the changed SQL files. Earlier neutral-planner and FileDb regression results
remain attributed to their own recorded earlier heads.

The model repeats the complete stored row size on every physical part, matching
`CONFIG_ROW_PART_BYTES` / `write_bulk_stage_rows_in_parts` and the documented
platform import convention in `docs/import/patch-mode.md` section 7. This is
source evidence for the representation, not a fresh SQL measurement: acquisition
must still reject an unexpected measured layout before compiling or publishing.
Selection merging removes all prior parts for the replaced names, including a
stale multipart tail, while preserving CONFIG and every unselected saved header.
An individually admissible replacement is refused if the complete merged readset
would exceed its limits. Cancellation is checked for merge name/key traversal.
Independent scoped static review approved this model. The 14 integration tests
and library check passed; actual SQL acquisition/publication and native acceptance
remain pending. No runtime database command was performed by these tests.
