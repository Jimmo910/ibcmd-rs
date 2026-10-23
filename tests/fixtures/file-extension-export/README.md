# Clean-room file extension snapshots

Generated on macOS with native 8.3.27.2214, 2026-10-23, in new disposable databases. No user database or application was copied.

- two-extensions.1CD.gz: empty native database; repository external/test_extension/input.cfe loaded as ТестРасширение and applied; native extension create added Second520 with prefix Second520_ and purpose add-on.
- borrowed.1CD.gz: base produced by base_sources from _onecdec/make_adopted_fixtures.py; native import/apply, then external/adopted/module_all/input.cfe loaded/applied as ТестРасширение.
- prepared.1CD.gz: copy after the borrowed CFE load and before apply; refusal without guessing the prepared image.
- *-sha256.json: every relative file path and SHA-256 from native export of the corresponding exact snapshot, including ConfigDumpInfo.xml. Native XML trees remain in the local lab; none is committed here.

Gzip mtime is zero. Tests unpack fresh unique directories under explicit TMPDIR or the repository target/test-fixtures fallback, clear PATH, compare full file sets/hashes and unchanged database bytes. IBCMD_TEST_KEEP_FIXTURES=1 retains fixtures. See the OpenSpec physical-profile evidence.
