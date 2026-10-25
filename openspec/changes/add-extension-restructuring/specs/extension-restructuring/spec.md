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
