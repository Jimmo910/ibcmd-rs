## ADDED Requirements

### Requirement: Extension update creates data-bearing objects atomically
The updater SHALL support the measured additive own-catalog and independent-information-register extension updates on 8.5.1.1150 and 8.3.27.2214, with corresponding schema, names, registry and cache publication.

#### Scenario: Multiple new objects
- **GIVEN** a usable versioned extension with existing control records
- **WHEN** import and apply add multiple own catalogs and information registers
- **THEN** a fresh platform session can write and read the new objects
- **AND** existing records, references, main configuration and unrelated extensions remain intact
- **AND** native re-export and subsequent native apply confirm a consistent active configuration

### Requirement: Failed updates preserve the prior database
The updater SHALL reject unsupported or destructive plans before writes and SHALL roll back all DDL, metadata and data effects on failure or trial execution.

#### Scenario: Failure during publication
- **GIVEN** a planned extension schema addition
- **WHEN** execution fails after table creation or a concurrent registry/schema change is detected
- **THEN** no partial schema, metadata or data changes remain committed
- **AND** the previous extension remains usable

#### Scenario: Repeated application
- **GIVEN** an already applied extension image
- **WHEN** the same update is applied again
- **THEN** no duplicate objects, records or allocations are created

### Requirement: Accompanying data-owning objects are classified
The updater SHALL classify accompanying metadata by its measured physical effects, including non-predefined scheduled-job tables and their service mapping, and SHALL apply measured base data-separation contributions to extension-owned objects.

#### Scenario: Full application update
- **GIVEN** an extension update with new catalogs, registers, a common module, HTTP service, subsystem and non-predefined scheduled job
- **WHEN** its complete source tree is imported and applied
- **THEN** all required tables and metadata are published atomically
- **AND** existing control records and references remain available in a fresh session

#### Scenario: Unmodeled predefined job instance
- **GIVEN** a new predefined scheduled job requiring a persistent activation-condition instance
- **WHEN** its update is proposed without an evidenced instance writer
- **THEN** the command names the unsupported job before staging or publication
- **AND** no database mutation is performed

#### Scenario: Legacy structural container
- **GIVEN** an active or staged extension outside the measured structural format/profile pairing
- **WHEN** its update requires new SQL schema records
- **THEN** the updater reports the active and staged formats before staging or publication
- **AND** no manifest version is silently changed
- **AND** previously supported body-only overlays retain their prior format support

### Requirement: Direct SQL profile verification without an infobase account
Direct SQL import and apply SHALL NOT require an infobase application user or
password and SHALL NOT open an authenticated infobase session as a preflight.
The tool SHALL retain exact platform-build validation and live SQL-schema
validation, and SHALL state the evidence actually used for RAS registration.

#### Scenario: Protected infobase with SQL access
- **GIVEN** a registered infobase with application users and valid SQL access
- **WHEN** import or apply is invoked without infobase credentials
- **THEN** profile verification uses administrative summary data and SQL facts
- **AND** it does not call `rac infobase info` or request an application login

#### Scenario: Wrong platform build or absent registration
- **GIVEN** an unverified build or missing selected RAS registration
- **WHEN** the operation is prepared without application credentials
- **THEN** it refuses before staging and preserves existing data