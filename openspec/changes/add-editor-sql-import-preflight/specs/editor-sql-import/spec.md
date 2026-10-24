## ADDED Requirements

### Requirement: SQL preparation is bounded and read-only
The server SHALL prepare existing selected SQL main-configuration changes from
bounded original source bytes and an immutable target snapshot without target
DML. Unsupported changes SHALL fail before any target write.

#### Scenario: Unsupported or cancelled preparation
- **GIVEN** a source outside the admitted existing module/form cohort, an exceeded
  resource limit, or a cancelled request
- **WHEN** SQL preparation is requested
- **THEN** no target mutation is dispatched and no publishable token is returned.

### Requirement: Consent and plan binding precede publication
The server SHALL bind each one-use plan to actual SQL identity, connection
generation, source census and exact target readset. It SHALL require final
confirmation and a new preparation after dependency expansion.

#### Scenario: Unresolved references or declined confirmation
- **GIVEN** missing references, a declined dependency suggestion, or no final consent
- **WHEN** import is requested
- **THEN** CONFIG and CONFIGSAVE remain unchanged.

#### Scenario: Source or connection changes
- **GIVEN** a prepared plan whose source or connection binding changed
- **WHEN** publication is attempted
- **THEN** it is refused before target DML and requires fresh preparation.

### Requirement: Publication is same-session and preserves staging
The SQL adapter SHALL validate identity and the complete compiler readset and
CONFIGSAVE CAS under retained locks in the same session/transaction as target
DML. It SHALL preserve every unselected staged row.

#### Scenario: Concurrent target or staging change
- **GIVEN** target rows, database identity or pre-existing staging changed since prepare
- **WHEN** the publication transaction validates the plan
- **THEN** it refuses and leaves CONFIG and CONFIGSAVE exactly as found.

#### Scenario: Existing unselected stage
- **GIVEN** supported pre-existing staging for an unselected object
- **WHEN** a confirmed verified selected change is published
- **THEN** that staging remains byte-identical and only the selected staged keys change.

#### Scenario: Interrupted publication
- **GIVEN** failure before commit, or a lost response after possible commit
- **WHEN** the adapter reports the outcome
- **THEN** a pre-commit error rolls back, and an uncertain or published outcome is
  not falsely reported as cancellation with no writes.

### Requirement: SQL readiness requires actual acceptance
The SQL capability SHALL remain unadvertised until the guarded adapter and
actual disposable SQL acceptance are verified.

#### Scenario: Synthetic tests without SQL acceptance
- **GIVEN** only a recording backend and portable fixture tests passed
- **WHEN** capabilities or completion are reported
- **THEN** real SQL publication and native apply/export are still pending.
