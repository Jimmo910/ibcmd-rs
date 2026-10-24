# SQL status contract lab (#380)

This directory prepares a **public, bounded, sole-operator** SQL status smoke.
It never performs stage/apply, opens SQL connections, runs native tools or
changes consent settings. `verify_capture.py` only checks a collected sanitized
transcript. Its success always says `liveAcceptance: false`: a reviewer must also
check actual platform output, request captures and storage/source provenance.

Portable verifier checks:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/status-lab -p 'test_*.py' -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/status-lab/verify_capture.py /external/owned-run/sanitized-transcript.json
```

The synthetic `transcript()` factory in `test_verify_capture.py` is the schema
example. Its hashes, operator, lease and timestamps are invented public controls,
not evidence of an executed operation. Capture metadata (`generation`, phase,
lease, hashes and disposition) surrounds the actual status request/response; it
is not a proposed RPC wire-field extension. Use real recorded values for a smoke.

A transcript has one public fixture digest, exact product commit plus embedded
commit/binary digest, a one-hour maximum exclusive disposable-DB lease, opaque
hashed target/source identities, and exactly two external consent receipts (stage
and apply). Each receipt binds the owner's exact preview, source, target, product
and lease. It does not itself perform or authorize a writer. The status operator
must obtain actual human/owner authorization separately, never generate a consent
receipt from a status result. A claimed boolean is evidence metadata, not an
access-control mechanism or a substitute for the supported writer's own checks.

There must be one published `before`, `staged` and `applied` capture plus actual
cancel and retired-generation controls. Maximum 24 captures, 64 objects each,
300 seconds per read interval and 1 MiB per transcript. Within each read interval,
source and storage fingerprints stay unchanged. Across intentional stage/apply
writes, storage may change; source identity stays fixed. Reference identity,
object/cache token and database snapshot must bind every native evidence row.
Unknown/full-required/error/cancellation cannot be submitted as conclusive clean.
The verifier does not derive labels or compare an invented native oracle.

Future operator sequence and explicit unproved boundaries:
[SQL acceptance](../../openspec/changes/add-vscode-object-status/sql-acceptance.md).
The isolated Rust suite is `tests/editor_sql_status_contract.rs`; run it only
with a planner grant and an external target/TMPDIR. Locally always set
`IBCMD_TEST_KEEP_FIXTURES=1`; owned synthetic fixtures remain for Trash-only
cleanup. CI may clean only its freshly owned disposable fixtures by default.
No script removes artifacts, drops databases, or alters originals.

No live SQL, VM, TDS, authentication, native platform or editor-host acceptance
is established by these portable controls. Existing merged #484 evidence remains
separate. No issue closure or full CI is implied.
