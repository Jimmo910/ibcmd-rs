## ADDED Requirements

### Requirement: Export one named extension from a file infobase

The exporter SHALL read a file infobase without invoking the platform and
publish the native 8.3.27.2214 XML tree of exactly the named extension.

#### Scenario: A measured extension

- **GIVEN** a closed file infobase with an extension supported by the measured profile
- **WHEN** `infobase config export --db-path=<dir> --extension=<name> <output>` runs
- **THEN** every output path and byte SHALL match the native export, including ConfigDumpInfo.xml
- **AND** the data file SHALL remain byte-for-byte unchanged

#### Scenario: Two independent extensions

- **GIVEN** two extensions with distinct registry identities and CAS roots
- **WHEN** one is selected by name
- **THEN** only its manifest-reachable records SHALL contribute to its XML tree

### Requirement: Refuse unsupported or invalid input before publication

The exporter SHALL validate registry and CAS data and refuse unknown or
ambiguous states before publishing output.

#### Scenario: Missing extension

- **GIVEN** no extension with the requested name
- **WHEN** export is requested
- **THEN** the command SHALL return code 1 and identify the missing extension
- **AND** no result tree SHALL be published

#### Scenario: Invalid registry or content-addressed image

- **GIVEN** an unknown registry layout, duplicate selected name, corrupt CAS or missing referenced digest
- **WHEN** export is requested
- **THEN** it SHALL fail with a contextual error and publish no result tree

#### Scenario: Unmeasured prepared state

- **GIVEN** extension preparation whose selection semantics are not supported by measured evidence
- **WHEN** export is requested
- **THEN** it SHALL refuse before publication rather than silently export an active image

#### Scenario: Unsupported incremental options

- **GIVEN** an extension export with --base or --sync
- **WHEN** the command is validated
- **THEN** it SHALL refuse without publishing output
