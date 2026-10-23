# Native and private-copy evidence

Native executable: 1C 8.5.1.1592, x86_64 on macOS arm64 through Rosetta. Empty inventory additionally measured on 8.3.27.2214. All commands used disposable laboratory file databases, explicit external data/temp directories, and no user database writes. Exact commands, stdout/stderr and hashes are retained privately in the external laboratory.

The committed corpus is exclusively repository-owned tiny/clean-room data: real native imports for add/change/delete/rename, full XML and status/delta outputs, and owned-form recursion. The fixture manifest records hashes and command templates. Differential tests compare complete stdout/files/errors, not normalized XML. Configuration and Language versions can change on native import while their XML bytes stay identical; status compares stored versions independently of byte comparisons.

## Large private-copy measurement (pre-final build)

One warm run per operation; release build, two conversion/output threads, no cache flush or machine tuning. No proprietary names, source XML, database or CF are included here. The 4.51 GB laboratory copy remained byte-identical (SHA-256 before/after). Native inventory and our inventory were byte-identical (45,836,493 bytes). All 105,253 version rows had canonical owners, with zero unmapped entries.

| Operation | Elapsed seconds | Result |
|---|---:|---|
| Rust info | 28.623 | rc 0, byte-identical native inventory |
| Rust full export | 92.570 | rc 0, 125,083 files |
| Rust unchanged base | 30.035 | rc 0, no output directory created |
| Rust one-row delta | 55.922 | rc 0, exact native paths and bytes |
| Native one-row delta | 30.327 | rc 0 |
| Rust unchanged sync | 30.265 | rc 0, all file timestamps/sizes unchanged |

The changed reference alters one saved module version, not database content. Relative to this Rust full export, unchanged base is 3.08x faster and one-row delta is 1.66x faster. The native delta is faster than the Rust delta; these measurements do not establish general platform performance. A final-build recheck is recorded separately after final validation.


## PR review follow-up (2026-10-23)

The five inline review findings were reproduced and corrected: query input errors
are no longer disguised as output-directory failures; generic sync failures do
not reuse the success sentence's final period; info/status accept benign linked
parent directories while still rejecting linked final entries; failed staged
writes and renames clean up only the operation's owned staging file after closing
its handle; and the default-platform subprocess clears an inherited platform
override.

Native 8.5.1.1592 and the corrected queries produced identical inventory/status
bytes through an explicitly selected directory alias on a new repository-owned
tiny database. Its before/after SHA-256 was unchanged. Fifteen query integration
cases include missing/malformed references and databases, linked/dangling final
entries, missing nested parents, and directory-required spellings that must not
replace an existing sentinel file. Two unit cases inject a partial write failure
and exercise a failed rename, checking destination preservation and staging cleanup.
The query suite also passes with the parent environment's IBCMD_RS_PLATFORM=8.5.1.

Directory-alias resolution is restricted to the caller-selected info/status
output. Object containment and source descriptor guards are unchanged. This
path-based publication does not claim protection against hostile concurrent
ancestor replacement. The independent review identified and verified the fix for
terminal dot-component normalization before approving the changes subject to gates.
The additional unverified items-after-test-module note was confirmed with real
root Clippy; moving the unchanged inventory helper above the tests removes it.
The physical policy baseline and scanner remain unchanged.

Exploratory root `cargo clippy --lib --tests` also reports a pre-existing
`clippy::never_loop` denial in unchanged `src/mssql_apply.rs`; that broader
research-crate lint run is not green. The mandatory standalone workspace Clippy
command excludes the root crate and passes unchanged.

Local review-follow-up gates: formatting, all-target workspace check, required
standalone Clippy, 1,252 standalone tests (80 ignored), 427 root integration tests
(4 ignored), release build, strict OpenSpec and physical policy/self-tests pass.
The 15 query cases and both staging tests pass; the final helper reorder has an
additional check/query/unit recheck. Independent review found no remaining P1/P2.
Exact-head multi-platform workflow results are recorded in the PR separately.
