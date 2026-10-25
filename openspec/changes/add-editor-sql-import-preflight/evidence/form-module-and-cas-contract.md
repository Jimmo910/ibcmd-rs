# Document Form module snapshot compilation and exact readset predicate (#382)

## Implemented boundaries

`selected/form.rs` resolves Configuration -> Document -> Form from original typed
collections, rejects cross-parent/generated/child identity collisions before name
lookup, binds exact source paths and registered body keys, and overlays only the
Form module. All three codec profiles are constructed from one EffectiveProfile.
Every unexamined root family refuses: the current cohort is Document-only, not a
claim of general mixed-configuration or other-owner support.

The Form revision, layout, all trailing sections, parent/Form descriptors and
unselected physical staging remain unchanged. The existing SQL versions guard
permits only the selected body generation and header generation delta. Root
identity/inventory/budget checks were mechanically shared with CommonModule via
CapturedView; there is no diagnostic-observer dependency.

`Readset::require_equal` compares exact full target identity, schema, both tables'
physical coverage/keys/PartNo/DataSize, complete headers and raw bytes with chunked
cancellation. It is the pure CAS predicate; it owns no SQL session or locks.
The dedicated-session transaction/poison/rollback/COMMIT-unknown contract is pinned
in design.md. The minimal SqlClient/SqlSession seam and MSSQL owned-connection
adapter are implemented, default Unsupported elsewhere; no production route uses
them. Requests share one owned connection, poison before driver invocation and
refuse reuse after errors or a caught panic. A healthy explicit rollback is
terminal and must actually roll back an active transaction; failed streams are
not assumed safe for more SQL. This is not a publisher or evidence of atomicity.

## Focused verification

Rust 1.95.0, jobs=1, external target/TMP, KEEP_FIXTURES=1:

- `cargo test --locked -p ibcmd-rs --lib server::sql_import::selected:: --no-default-features`:
  17 passed / 0 failed / 0 ignored, 49.26 s compilation and 0.04 s test time.
  Includes 7 in-memory clean-room Form cases and the previous 10 module cases.
- After adding the readset predicate,
  `cargo test --locked -p ibcmd-rs --lib server::sql_import::readset::exact_readset_tests:: --no-default-features`:
  3 passed / 0 failed / 0 ignored, 60 s compilation and 0.04 s test time.
  Covers identity/schema/coverage/physical metadata/payload mutations and typed
  cancellation, including unselected staging changes.
- Integrated `cargo check --locked -p ibcmd-rs --lib --no-default-features`:
  exit 0, 6.66 s. Existing warning totals remain 40 lib-test / 97 production.
- Independent source reviews cleared Form ownership/overlay, the mechanical view
  extraction, the exact readset predicate and the revised session contract.
  The wrong test import found in source review was fixed before compilation.
- All test/compiler processes exited. All fixtures and logs are retained.

External receipt: `logs/issue-382-form-readset-receipt.json`, with source/log hashes.
The two filtered test runs remain attributed separately; they are not a real SQL
CAS, transactional rollback/commit or native apply/export acceptance run.

## Explicit remaining gates

Form.xml edits are not enabled. Independent source audit found that the legacy
packer can skip unresolved commands with source=None, while the diagnostic XML
leaf-map comparer drops some empty elements, qualified names and GeneralRef
information. Those helpers do not prove complete Form.xml consumption. They are
not used to widen this module-only capability or to weaken unknown-loss refusal.

The dedicated session seam has not been tested against a real SQL connection.
Held transaction/full readset CAS, selected-row native header/part encoding,
confirmation token routing and actual disposable SQL
stage -> native apply -> re-export remain unfinished. No SQL requests, VM/native
calls or database writes occurred. The existing prepare RPC still refuses SQL;
the legacy replace_config_save path is not a fallback for this code. Unknown
COMMIT outcomes must remain distinct from rollback and must not be retried blindly.
The diagnostic observer/baseline-registration blocker is unchanged, no baseline
or whitelist was expanded, and #382 is not complete.

## Dedicated-session source seam follow-up

`cargo test --locked -p ibcmd-rs --lib dedicated_session --no-default-features`:
4 passed / 0 failed / 0 ignored, 58.05 s compilation. These test the unsupported
default never invoking work, poison/no replay, terminal cleanup and caught-panic
state. They do NOT open a TDS connection or prove real rollback/commit behavior.
The exact active-transaction rollback guard and absence of production callers
were independently source-reviewed. No SQL driver/protocol state is guessed
healthy after failure. Publication outcomes remain the future transaction
adapter's responsibility; generic session success/error cannot classify COMMIT.

New source paths: `src/sql/mod.rs` and `src/sql/mssql/mod.rs`. Existing TLS/auth,
connection-establishment rules and ordinary pooled methods are unchanged. No
sqlcmd/legacy-stage fallback was introduced. No server method or CLI was added.

Integrated production lib-check after the session seam: exit 0 in 8.19 s, same
97-warning baseline. OpenSpec strict and formatting/diff checks passed. All local
Cargo/test/compiler processes exited; the compiler slot is FULL_RELEASE. The
external receipt was extended with the two SQL backend source hashes and session
logs. There were zero actual SQL connections in these pure session tests.


## Held Form source-file preparation

The filesystem FormModule wrapper shares the existing Document-only resolver and
codec path. It compares original held file bytes with the expected native module
file representation before invoking the existing loader storage policy. An exact
native BOM/CRLF file preserves raw module storage, layout, trailing fields and
versions; a real edit uses the same pure byte policy as existing Form loading.
The original storage-text API retains its prior BOM/EOL semantics. Source bytes
and their custody hashes are never normalized. SourceAdmission consumes one
exact retained path and revalidates before/after compilation; Form.xml and other
owner cohorts are not enabled.

Four new wrapper tests cover native-file no-op, changed-file loader policy with
exact non-module state, expanded budget 7 refusal/8 acceptance, and typed late
cancellation. One admission integration covers retained bytes, no-op snapshot
identity, wrong selected path and typed source-mutation refusal. Its Form.xml
path case proves outside-selection refusal; canonical Form.xml role refusal
remains covered by the pre-existing resolver/bridge cases.

Rust 1.95 local focused suite: 27 passed, zero failed/ignored (40.35s compile,
0.06s tests). Compiler/transaction compatibility: 33 passed, three live tests
ignored; production library check passed (6.01s). Existing warning totals remain
40 lib-test and 97 production. Independent source reviews approved the two-file
wrapper donor b524488b and the separate admission/test consumer; the latter had
only rustfmt layout adjustment afterward. Formatting and diff checks passed.
External logs: issue-382-held-form-source-focused.log,
issue-382-held-form-stage-compat.log, issue-382-held-form-libcheck.log.
No SQL/VM/native test ran in these gates and the editor SQL route stays closed.
