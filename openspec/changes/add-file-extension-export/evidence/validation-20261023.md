# Validation, 2026-10-23

Implementation commit: `e1e9a720dc5a9d378084a5e3c0c84eac38d580f1`, based on `1be98ec77a8096553915d555cdd5ffb10fe34e99`.

## Native differential

Exact native utility: **8.3.27.2214**, confirmed by `ibcmd --version` before fixture preparation. Native executable SHA-256: `1a35bba7883a7fdf9a8678392e4b1520df452995ce005ff225f94594e70cfe25`.

The utility only created/loaded/applied/exported new clean-room databases. All work/data/temp paths were in the dedicated external lab. No original/user infobase was used. Production follows the file reader/CAS/XML path without a process invocation.

| Selected image | Full native files | Byte parity including ConfigDumpInfo |
| --- | ---: | --- |
| Own extension from a two-extension DB | 5 | exact |
| Second extension from that same DB | 3 | exact |
| Extension borrowing a common module | 7 | exact |

Native before/after database SHA-256:

- Two-extension DB: `f69074d092c97149f534fb2aefd318c0f6d5d3a5fec7324dd9ce39528fb356da`.
- Borrowed-object DB: `24b5fd4ba179769f1a56ff24c38455be398c72c3fab3a9a484cda9d43379bff6`.

Both hashes remained unchanged after native exports and the Rust comparisons. Native unknown-name export returned 1 and created no output. Each portable Rust parity case also compares the complete database bytes before/after and asserts no backup was created.

The initial iter differential was bound to binary `875e7b73214d368ae20051366ed0c77078f66f2abd0af308eca13af984c9a8a6` and source-patch SHA-256 `88ea5971e838f425191b40676317e74b8ae80a19150ad96cdfbc9d3971e29820`. It proved the three XML trees, then exposed inherited CLI exit 255 for refusal. That result is not attributed to a later binary.

The final safe check used debug product binary SHA-256 `dd500d2cb65b37311b066de0837ef045ca6ff0f289e74720b2dfd16d9c86fa06`. Its ten end-to-end cases include the same complete native hash inventories, code-1/no-output refusal, prepared-state refusal, corrupt file/envelope/CAS, physical part-number overflow and preservation of nonempty output. The CLI reports version 0.4.1; it has no embedded Git commit, so source commit and binary hashes are recorded explicitly.

## Reproducible local checks

Pinned Rust **1.95.0**, jobs **1**, tests serialized, external CARGO_HOME/RUSTUP_HOME/target/TMPDIR from lab-env.sh, `IBCMD_TEST_KEEP_FIXTURES=1`. Three scoped legacy test helpers now retain unique fixtures under this flag; the broad retention draft was never applied.

- `cargo xtask check --base 1be98ec77a8096553915d555cdd5ffb10fe34e99`: all 6 steps passed, including workspace-crate strict Clippy and root all-target checking.
- `cargo test --locked -p ibcmd-filedb`: **16 unit + 8 integration**, 0 failed/ignored.
- `cargo test --locked -p ibcmd-rs --test file_extension_export --no-default-features`: **10 passed**, 0 failed/ignored.
- `cargo test --locked -p ibcmd-rs --test file_infobase --no-default-features`: **6 passed**, 0 failed/ignored.
- Root CFE suites `extension_adopted`, `extension_adopted_properties`, `extension_export`: **8 + 11 + 11 passed**, 0 failed/ignored.
- Root unit filters `mssql_extensions::tests` and `mssql_dump::cas::tests`: **12 + 14 passed**, 0 failed/ignored.
- **96 distinct focused tests passed**. SQL coverage is offline registry/CAS contracts, not a live SQL acceptance claim.
- `OPENSPEC_TELEMETRY=0 npx --offline --yes @fission-ai/openspec@1.6.0 validate add-file-extension-export --strict --no-interactive`: valid.

Failing-first evidence: the measured-registry test first refused the supported layout. The actual CLI first returned 255 for an unknown name. The physical N(10) part value 2^32 first wrapped to zero and returned success; the new regression failed with `left: Some(0), right: Some(1)` before the bounded-reader fix. It now refuses before publication.

## Fixture provenance

The compressed snapshots and per-file native SHA-256 manifests are in tests/fixtures/file-extension-export. They derive solely from the repository's clean-room CFE/source builders; see its README. Borrowed CFE input SHA-256: `0c9c2f5267ebe41708a86a45d8cc6c72f5531eb6fee94d10b1d89de6705d99c6`.

| Snapshot | gzip SHA-256 | Uncompressed 1CD SHA-256 |
| --- | --- | --- |
| two-extensions | `6565fc65942994f1865de4321b10b32f24e66abf6859815a37b60cd1e9d850e5` | `f69074d092c97149f534fb2aefd318c0f6d5d3a5fec7324dd9ce39528fb356da` |
| borrowed | `9e69a778647117d37b9b42795d56353cca7beecf9c25148721fe0bc877d1e91c` | `24b5fd4ba179769f1a56ff24c38455be398c72c3fab3a9a484cda9d43379bff6` |
| prepared | `a149a89a2e61bd0dc8b3255abf9f555a7be1cec726a15f0b2f45e91d859252f0` | `f2ab7e05bb5a4cb8410fb9f71b62654a1a36cb41d459249720371bdb1412ce6d` |

## Remaining gates and supported boundary

This initial file profile exports the applied image only when CONFIGCASSAVE and CONFIGSAVE are empty. Prepared snapshots are deliberately refused; prepared/mixed image support is not claimed. No all-extension/list/import/apply/write/SQL-online/8.5/EDT/writer changes are included.

Full local `cargo xtask ci` was **not run**: unrelated legacy Rust/Python/PowerShell teardown does not consistently retain fixtures. Automatic approval review also rejected the entire proposed local batch containing PowerShell policy/corpus guards for potential system-temp creation/deletion. That rejected batch did not execute; no local guard retry occurred. The scanner and policy baseline are unchanged. Safe Cargo-only checks above subsequently ran successfully. Full gates/guard self-tests on unrestricted disposable CI runners and upstream required checks remain pending; none is claimed passed here.

Independent Rust review and PR CI are separate pending gates at this evidence revision. The PR remains Draft; Refs #520 and #348, with no automatic issue closure.

Current upstream advanced to `1688c73b9bef00df31aa87aa0f259a1133fc3624` (#519) during validation. The comparison adds the 1.95 toolchain pin, documentation/temp cleanup and a test include-file relocation; no production Rust change overlaps this feature. The feature has not cherry-picked the parallel unpublished #520 implementation. CI must validate the actual PR merge context.
