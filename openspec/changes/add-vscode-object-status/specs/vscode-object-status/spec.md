# VS Code object status

## ADDED Requirements

### Requirement: evidence-backed six-state labels
The client SHALL label agreement, DB edits, file edits, edits on both sides, DB-only
and file-only objects using explicit source and synchronization baseline evidence.
It SHALL show unknown when direction cannot be established, including a DB version
change between comparison responses. Identical concurrent edits SHALL remain both.

#### Scenario: independent native and local changes
- **GIVEN** a conclusive native version comparison and a readable synchronization reference
- **WHEN** DB versions and local bytes independently change or remain equal
- **THEN** the client distinguishes same, database, file and both
- **AND** missing local or DB objects produce databaseOnly or fileOnly respectively
- **AND** missing or stale evidence for common objects produces unknown

### Requirement: minimal object UI and Explorer
The client SHALL expose a minimal object list, matching source Explorer decorations,
changed-only filter and explicit refresh. Unknown SHALL remain in changed-only mode.

#### Scenario: changed-only view shares Explorer labels
- **GIVEN** the six states are visible after a refresh
- **WHEN** the user enables changed-only filtering
- **THEN** same objects are excluded and unknown objects remain
- **AND** source Explorer decorations retain the same object labels

### Requirement: baseline choice
The selected workspace folder SHALL independently configure a read-only baseline
folder or a pinned Git tree and explicit cache path. Git SHALL materialize only the
source subtree and SHALL reject unsafe paths/symlink entries without executing a shell.

#### Scenario: pinned Git reference
- **GIVEN** a selected workspace source subtree and explicit external cache directory
- **WHEN** a Git ref is used as baseline
- **THEN** the client resolves one tree and reads only that subtree without checkout or shell
- **AND** matching native results agree with a folder containing the same baseline export

### Requirement: lifecycle safety
Cancelled/failed/old refreshes SHALL NOT publish clean or restore another selection's
badges. Configuration, connection, crash and workspace removal SHALL clear evidence.
Unrelated workspace folders SHALL NOT inherit decorations.

#### Scenario: late response cannot restore another selection
- **GIVEN** a refresh for one workspace/connection is pending
- **WHEN** the user selects another connection or removes that workspace
- **THEN** outstanding work is invalidated and its late response cannot publish decorations
- **AND** cancellation or failure clears previous evidence instead of reporting clean

### Requirement: native acceptance remains explicit
Rows-dir and synthetic UI evidence SHALL NOT be described as native filedb parity.
Native acceptance SHALL use the shared backend and measured platform status contract.
Native mode SHALL NOT silently fall back to byte equality. Unmapped owner coverage
and incoherent source views SHALL be inconclusive or refused. Native fullRequired
SHALL be preserved explicitly. Both comparison responses SHALL bind the same
native snapshot and reference identity.

#### Scenario: native filedb acceptance and unsupported coverage
- **GIVEN** a disposable filedb changed through native selected-file import and separate local edits
- **WHEN** the actual extension host refreshes against the synchronization baseline
- **THEN** its six tested states agree with measured native versions and local bytes
- **AND** fullRequired, unmapped roles and incoherent source views cannot become clean
- **AND** native command evidence is not described as Configurator GUI validation
