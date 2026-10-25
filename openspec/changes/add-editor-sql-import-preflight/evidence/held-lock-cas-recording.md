# Held-lock CAS recording checkpoint (#382)

## Local implementation and evidence

The library-only transaction adapter uses one dedicated session, acquires the
existing master-scoped LIVE guard, then holds CONFIG followed by CONFIGSAVE table
locks through the full identity/schema/header/part/payload comparison, selected
staging updates, post-write comparison and COMMIT. Unselected physical staging is
retained; selected metadata comes from the captured effective part-zero row.
The existing readonly capture still rolls back and uses its original seal.

Cancellation is checked after the last synchronous guard immediately before
COMMIT dispatch. Acknowledged COMMIT remains committed despite later cancellation;
a lost COMMIT acknowledgement is Unknown with no automatic retry. A rollback
acknowledgement and an uncertain/poisoned connection are separate outcomes.

Four population parameter values are summed with SqlParam::wire_bytes and checked
addition. This budget excludes SQL text and protocol framing; it is not a complete
TDS request-size bound. No native multipart/header writer profile is asserted.

Rust 1.95.0, one compiler job, external target/temp and retained fixtures:

- transaction::tests: 11 passed, zero failed/ignored; 1m29s compile, 0.01s tests.
  Includes exact parameter-budget minus-one/exact/plus-one, cancellation raised
  inside the final guard, XACT_STATE=-1, SPID=0, stale readsets, preserved
  neighbouring staging, rollback failure, commit-response loss and late cancel.
- editor_sql_import: 29 passed, zero failed, one real-SQL test ignored.
- production library check: exit zero, 16.65s; existing 97 warning headings.
- Independent read-only source review: CLEAR. No test or source review opened SQL.
- Legacy LIVE rendered format template was expanded and compared against the
  parent checkpoint: byte-identical. Canonical snapshot.sql remains unchanged.

Logs and hash receipt: logs/issue-382-held-cas-{mock,snapshot-regression,check}.log
and logs/issue-382-held-cas-receipt.json in the external lab workspace. Existing
failed-run artifacts and foreign package sources are retained.

## Next disposable-SQL gate

Only the existing sole operator executes VM/SQL work. First revalidate the exact
public disposable target GUID, server/family/fork/schema and certificate-valid
product TDS identity. Use the existing ordinary invocation path, not the retired
custom launcher. The next integration must prove a dedicated session keeps the
same positive SPID and one committable transaction while acquiring CONFIG then
CONFIGSAVE, comparing the complete captured readset and rolling back without
application DML. Confirm closed connection/locks and unchanged database afterward.

Before an application write, separately review an explicit disposable-fixture
integration invoking the snapshot-only CommonModule compiler and this adapter.
Prove stale full readsets produce zero application DML; successful staging keeps
all unselected CONFIGSAVE headers/parts byte-identical; selected multipart/header
encoding is accepted by native apply; native re-export matches the intended edit.
A transport-disconnected COMMIT is Unknown, never retried blindly. A busy or
uncertain earlier operation stops the next attempt. No working ERP/UT target is
eligible. This checkpoint does not dispatch that integration or invent a writer
profile from recording tests.

There is no CLI/RPC/MCP write-route caller. The public library stage function is
not technically mock-restricted; the executions reported here use recording
clients only. Form.xml remains unsupported; Form module support is the existing
Document-only cohort. Full #382 acceptance, confirmation routing, real SQL
atomicity and native stage/apply acceptance remain incomplete. The diagnostic
observer branch policy-registration failure remains open; no baseline or guard
was changed and this branch is not publication-ready.
