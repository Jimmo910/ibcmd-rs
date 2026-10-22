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
