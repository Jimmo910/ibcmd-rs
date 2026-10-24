## ADDED Requirements

### Requirement: Public self-service ownership
The bot SHALL accept `/claim` only for an open unlocked unassigned Ready issue in the configured project, assigning only the verified command author, setting In progress and recording its own Agent line while preserving other owners' text.

#### Scenario: External contributor takes a task
- **GIVEN** a human commenter without collaborator access and a free Ready issue
- **WHEN** the commenter submits `/claim`
- **THEN** assignment and both project fields are read back before a successful receipt is published.

#### Scenario: Task is not available
- **GIVEN** another assignee, an unavailable issue state or an unknown/missing project profile
- **WHEN** `/claim` is submitted
- **THEN** it is refused without an Assignees/Status/Agent mutation.

### Requirement: Owner-only release and review
The bot SHALL permit only the verified sole owner to release a task or request review of their own open non-Draft PR in the same repository targeting master and explicitly referencing the issue. It SHALL NOT close issues, merge PRs or grant access.

#### Scenario: Release and review
- **GIVEN** the requester is the sole owner
- **WHEN** `/unclaim` or a valid `/review #PR` is submitted
- **THEN** the corresponding Ready/unassigned or In review/assigned state is verified and acknowledged.

#### Scenario: Foreign ownership or PR
- **GIVEN** a foreign owner/author/repository, a draft/closed/unrelated PR or malformed command
- **WHEN** a command is submitted
- **THEN** it is refused without changing tracking state.

### Requirement: Serialized durable command processing
The bot SHALL serialize commands per issue and create a digest-bound durable receipt before tracking mutations. Duplicate or stale commands SHALL NOT replay. Every tracking write SHALL use fresh guarded state and verify its result. An uncertain or partial operation SHALL NOT produce a success receipt.

#### Scenario: Concurrent and repeated claims
- **GIVEN** competing command deliveries or a repeated delivery
- **WHEN** requests are processed
- **THEN** a confirmed owner is never displaced and a recorded request is not applied again.

#### Scenario: Drift or API failure
- **GIVEN** tracking state changes externally or an API write fails
- **WHEN** the operation is checked
- **THEN** further tracking writes stop, the receipt records refusal/error where possible, and no successful confirmation is emitted for an unverified state.

### Requirement: Trusted execution and separate project authentication
The workflow SHALL execute only trusted default-branch code for commands, treat comments as data, and isolate repository and project credentials. PR checks SHALL have no project secret or write permissions. Missing credentials SHALL refuse tracking mutations.

#### Scenario: Missing project credential
- **GIVEN** PROJECT_TOKEN is absent or cannot access the configured Project
- **WHEN** a command is received
- **THEN** no Assignees/Status/Agent write occurs and the contributor receives a configuration error where repository comment access permits.
