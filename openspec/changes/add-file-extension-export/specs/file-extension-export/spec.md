## ADDED Requirements

### Requirement: Named read-only file extension export
The command SHALL export exactly one named extension from the supported measured 8.3.27.2214 file profile without invoking the platform or modifying the database.

#### Scenario: Own extension
- **GIVEN** a supported synthetic file database with an extension owning metadata
- **WHEN** `infobase config export --db-path=<dir> --extension=<name> <output>` succeeds
- **THEN** its full XML tree including ConfigDumpInfo SHALL match native 8.3.27.2214 byte for byte and the database hash SHALL remain unchanged.

#### Scenario: Borrowed object
- **GIVEN** an extension borrowing an object from the base configuration
- **WHEN** its named export succeeds
- **THEN** the full XML tree including ConfigDumpInfo SHALL match native output using base context from that same database.

#### Scenario: Two independent extensions
- **GIVEN** a supported database containing two extensions
- **WHEN** one exact name is requested
- **THEN** only that extension's manifest-reachable logical files SHALL be rendered, without CAS mixing.

### Requirement: Refusal before publication
The command SHALL fail before publishing output when selection, physical profile, prepared state, CAS integrity or exact XML conversion cannot be established.

#### Scenario: Unknown name
- **GIVEN** no extension with the requested name
- **WHEN** export is requested
- **THEN** exit code SHALL be 1 with an understandable error and absent output SHALL remain absent.

#### Scenario: Unsupported or corrupt profile
- **GIVEN** an unsupported registry layout/envelope or corrupt selected CAS graph
- **WHEN** export is requested
- **THEN** the command SHALL fail without publishing output or changing the database.

#### Scenario: Unknown prepared state
- **GIVEN** extension preparation state that the measured reader cannot interpret exactly
- **WHEN** export is requested
- **THEN** it SHALL refuse before publication rather than substitute the active image.

#### Scenario: Incomplete XML
- **GIVEN** selected storage that the existing XML exporter cannot represent exactly
- **WHEN** rendering reports opaque or failed entries
- **THEN** output SHALL not be published.
