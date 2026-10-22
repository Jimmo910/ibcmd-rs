# VS Code object status

## ADDED Requirements

### Requirement: evidence-backed six-state labels
The client SHALL label agreement, DB edits, file edits, edits on both sides, DB-only
and file-only objects using explicit source and synchronization baseline evidence.
It SHALL show unknown when direction cannot be established, including a DB version
change between comparison responses. Identical concurrent edits SHALL remain both.

### Requirement: minimal object UI and Explorer
The client SHALL expose a minimal object list, matching source Explorer decorations,
changed-only filter and explicit refresh. Unknown SHALL remain in changed-only mode.

### Requirement: baseline choice
The selected workspace folder SHALL independently configure a read-only baseline
folder or a pinned Git tree and explicit cache path. Git SHALL materialize only the
source subtree and SHALL reject unsafe paths/symlink entries without executing a shell.

### Requirement: lifecycle safety
Cancelled/failed/old refreshes SHALL NOT publish clean or restore another selection's
badges. Configuration, connection, crash and workspace removal SHALL clear evidence.
Unrelated workspace folders SHALL NOT inherit decorations.

### Requirement: native acceptance remains explicit
Rows-dir and synthetic UI evidence SHALL NOT be described as native filedb parity.
Native acceptance SHALL use the shared backend and measured platform status contract.
