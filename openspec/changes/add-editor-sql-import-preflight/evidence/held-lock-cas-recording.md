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

## Operator-only held-capture test prepared

`transaction_tests.rs` now contains one ignored test:
`server::sql_import::transaction::tests::public_sql_held_capture_rolls_back_and_closes_dedicated_session`.
No production API changed. A Windows-only gate and explicit existing disposable
GUID/endpoint opt-in precede product TDS/TLS connection creation. The profile and
bounded initial snapshot must match the existing public 8-part/3313-byte fixture.
The existing pooled reader remains open while one owned dedicated connection
acquires the normal LIVE guard, begins a transaction and performs two held
captures. Both seals require TX depth 1, XACT_STATE 1 and the same positive SPID.
Both complete readsets equal the initial capture. One terminal rollback is
attempted even if a comparison fails; PASS requires acknowledgement.

After the dedicated callback returns, the pooled reader requires an idle session,
a distinct SPID, existing VIEW SERVER PERFORMANCE STATE permission and zero DMV
rows for the old dedicated SPID. It does not grant that permission. A fresh
bounded capture must still equal the entire initial physical readset. No temp
population, application DML, COMMIT or native call is in this ignored test.

Local Rust 1.95.0 test compilation passed in 40.64s: 11 recording tests passed and
this one live test remained ignored. Independent source re-review CLEAR after
adding the DMV visibility requirement. Existing 40 lib-test warnings unchanged;
no source-owned warning. Log: logs/issue-382-held-capture-compile.log, fixtures
retained. This is Mac compilation, not Windows execution or real SQL acceptance.

Once an actual Windows lib-test executable with checked source commit and SHA256
is available, the sole operator uses the existing ordinary SSH/PowerShell path:

```powershell
$env:IBCMD_SQL_IMPORT_PUBLIC_FIXTURE = '9000716B-B12F-45BE-BB5B-27FB88534F95'
# Endpoint and existing certificate configuration come from the current lab
# binding; do not substitute a guessed endpoint or certificate bypass.
& .\ibcmd_rs-lib-test.exe --ignored --exact server::sql_import::transaction::tests::public_sql_held_capture_rolls_back_and_closes_dedicated_session --nocapture --test-threads=1
```

Before dispatch, verify the exact hash/source binding, current identity/schema,
TLS, authorized integrated principal, idle window and absence of an uncertain
previous attempt. Require exactly one test passed; zero tests is not acceptance.
Raw failure details remain VM-local. No automatic retry or write-route enabling.

Windows artifact preparation is blocked: automatic approval review rejected the
proposed change that would upload the test executable and build metadata through
the existing Windows CI artifact step. The workflow edit did not execute; the
workflow is unchanged, with no push/dispatch or alternative artifact transport.
The explicit reason was missing trusted end-user authorization for that exact
external artifact egress/workflow side effect. A new specific authorization is
required before retrying that action. Windows Rust/MSVC availability on the lab VM
is not established; no toolchain was installed or new launcher created.

The artifact-upload blocker above is historical: a subsequent explicit approval
covers the exact existing fork workflow edit, push, Windows build and executable/
SHA256/source-commit artifact download for this disposable-lab test. The minimal
existing Portable (Windows) job addition packages only one Cargo-reported root
lib-test executable and its build receipt in a fresh RUNNER_TEMP subdirectory.
It does not execute ignored SQL tests, change permissions/secrets or upload
application XML/database data. PowerShell AST check and independent source review
passed before dispatch. This authorization does not establish runtime PASS or
make the diagnostic branch policy gate pass.

## First Windows CI result and checkout fix

CI run 40186940076 at 53ca54e17cff6cd280dad39098911ebefb4d03ca completed:
Linux and Legacy Windows passed, but Portable Windows root tests had 4263 passed,
6 failed and 17 ignored. The six new recording failures all stopped at
`canonical snapshot control anchor changed`; the upload steps did not run.
The original failed log is retained as issue-382-windows-ci-first-failure.log.

Git's `core.autocrlf=true cat-file --filters HEAD:src/server/sql_import/snapshot.sql`
reproduced the cause locally: 7101 bytes, 110 CRLF pairs, SHA256
50b80805d1942e51f9946a7e180bd2cdf864fd4133c1de93d513be684249c928,
and zero matches for the three strict newline-bearing control anchors.
A single scoped `.gitattributes` rule pins this embedded SQL file to `text eol=lf`.
The same checkout-filter control then returns exactly the canonical 6991 bytes,
zero CR bytes and source SHA256 e6d1ecbfe06cd2df605cf53066a425dd98058056a701b661786567c057056da6.
All original anchor counts are restored (1/1/1 and two table-lock anchors).
No parser, SQL, guard or test assertion changed. Independent source review CLEAR;
the same local CAS suite passes 11/11 with the one live test still ignored.
Actual corrected Windows CI/runtime remain separate pending observations.

## Measured server-permission invocation correction

Corrected checkout CI 40189346243 at 0746b58aff692381c97cca0d1f6a5ffebb3d29b6
passed all seven jobs. Windows root tests: 4269 passed, zero failed, 17 ignored.
Artifact 12598286457 contained only the executable and build receipt; source/tree,
Rust 1.95.0, x64 PE and executable SHA256 were checked. The retained first EXE is
0d7e39ec9340572cf196ddebb475d2600424383147b4314bc83db9d21e82e7fe.

The sole operator stopped at SOURCE_PREFLIGHT before launching that EXE. One
preflight connection closed; no held transaction, appDML or native call ran.
A bounded own-permission SELECT then measured NULL for
HAS_PERMS_BY_NAME(NULL,'SERVER',...), but 1 for the documented server-level
HAS_PERMS_BY_NAME(NULL,NULL,...). The existing caller was
WIN-LHDQGRLB1L1\Administrator, with sysadmin=1 and CONTROL SERVER=1; the permission
was recognized. This was an invalid function invocation, not a missing right.
No GRANT or other security change is needed or performed. Diagnostic FULL RELEASE
SHA256: bf9679f2ceb998b80778d27d0f280bf19288c11960d1983209ec13873a72a074.

The runtime closure now shares a canonical NULL,NULL query with the regression.
Visibility still must equal 1; invalid NULL and denied 0 fail distinctly, and all
idle/distinct-SPID/closed-session checks remain strict. Tests also refuse an
existing session, the reader's own SPID, and a nonpositive held SPID. No production
API changed. Local focused tests: 12 passed, zero failed, one live test ignored;
38.42s compilation, 0.02s execution. Independent source review CLEAR. The operator
owns the matching preflight correction. The old executable must not be used;
a fresh changed-head Windows artifact and actual held-capture runtime remain
required. Raw SQL and failure artifacts remain retained in the lab.
