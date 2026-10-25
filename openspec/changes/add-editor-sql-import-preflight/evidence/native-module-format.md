# Public SQL native module formatting

The retained full export receipt `811808513f267f44a801bed23d33ad9943e4545c845e8ce64055666b7d7181f6` compares product `1157133354b14a012e9c3e8fd9bbe58b2bef01b2` with the existing native 8.3.27.2342 public fixture. Product profile is the supported release `8.3.27`; exact-build registry support is not claimed. The product exited naturally with code 0, performed no SQL writes or native calls, and preserved source fingerprints. Four of six files matched exactly, including all five CDI tuples and versions.

`CommonModules/Portable/Ext/Module.bsl` was 38 bytes with LF and no BOM; native was 43 bytes with UTF-8 BOM and two CRLF line endings, SHA-256 `f6c91e7f53f80656a0a726cf13fc3ae8fdfd54ace2fbd26069885a101b611e48`. Removing only BOM and normalizing line ends made these two modules equal. Existing checked-in native-export-command module fixtures also use BOM and CRLF. The stored text decoder intentionally preserves its text-member bytes; this is an output-format issue rather than evidence of lost BSL content.

The narrow fix formats modules at filesystem source emission only. It preserves an existing BOM/CRLF, adds no final newline, and leaves lone CR and all other bytes unchanged. Raw storage/debug exports and memory sinks used by the editor/compiler retain their original bytes. Only files already selected for emission are affected; no post-export directory rewrite is performed.

The three focused writer regressions cover the exact 43-byte public native oracle, Unicode and idempotence, and raw/memory preservation. All three passed on Rust 1.95 with jobs=1 (`logs/issue-382-native-bsl-tests.log`); the filtered run executed only these three tests, retained their fixtures, and passed a subsequent root lib-check (`logs/issue-382-native-bsl-check.log`). Independent source review found no blocker. A fresh Windows product export remains necessary before claiming this native difference is closed.

The seven ChartPolicySeed template text differences are separately owned by #529 and are not changed here. This fix does not establish full-tree parity, SQL import authority, same-session CAS, or publication readiness. Raw SQL rows and native XML remain in the VM.

The full branch physical-adapter guard currently refuses the inherited, unchanged `src/mssql_dump/import_evidence.rs` donor as an unbaselined file (path SHA-256 `5574a62490168322556d12d020d387ae765a939f0f12587067ea69e930825999`). That separate gate was reported to its owner; no baseline was edited or check suppressed. This three-file module-format donor excludes the observer. The guard self-test was not run locally because it permanently removes its temporary fixtures, contrary to the lab retention constraint.

## Integrated Windows runtime result

The unchanged native baseline was compared with common product `c45d1bb2a9ea260dd5d7bb32f3143eea7ab29076`, executable SHA-256 `3f2ae372b411e1c552395ca5745c9e46b31c2d43fe0121ab6fde32e446e4df32`, workflow `40151625588`. Independently verified combined release receipt SHA-256 `3ead95f368c8419a67644b601e82aa3b5a0702fc07f439cad7985edd7ab5ccda` records a natural code-0 full public SQL export in 2.848 seconds, no forced termination, no SQL writes or native invocation, unchanged source fingerprints and FULL RELEASE.

`Module.bsl` now matches native exactly: 43 bytes and SHA-256 `f6c91e7f53f80656a0a726cf13fc3ae8fdfd54ace2fbd26069885a101b611e48`. ConfigDumpInfo is also byte-exact, including all five ordered ID/version tuples. The complete SQL comparison is **5/6 byte-exact, not a full pass**: ChartPolicySeed's template still differs at seven text positions and is owned by #529. Missing and unexpected file counts are both zero.

The status/selected-export follow-up remains required: its expected filesystem representation must share the formatter while memory/source-read and actual source custody hashes remain raw. This runtime result validates the original filesystem-format donor included in `c45`; it does not establish runtime acceptance of later helper/consumer commits, SQL import, CAS or publication.
