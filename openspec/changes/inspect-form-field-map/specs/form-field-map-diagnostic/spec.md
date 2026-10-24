## ADDED Requirements

### Requirement: Bounded existing-parser inspection
The diagnostic SHALL inspect one local Form body using existing parser/resolver
logic with compressed, inflated and report bounds, without network or writes.

#### Scenario: Valid structural observation
- **GIVEN** one supported bounded Form body and dynamic-list settings
- **WHEN** the diagnostic completes
- **THEN** it reports requested duplicate, required, ordering and alias facts
- **AND** it reports unavailable context as unknown rather than inferring a cause

#### Scenario: Unsupported or oversized input
- **GIVEN** malformed, unsupported, oversized or ambiguous input
- **WHEN** inspection cannot establish the requested facts safely
- **THEN** it refuses or marks those facts unknown
- **AND** it does not publish a partial successful report

### Requirement: Non-disclosing output
The diagnostic SHALL emit only a fixed typed schema of booleans, counts and
anonymous ordinals; failure diagnostics SHALL be static non-sensitive codes.

#### Scenario: Source contains private sentinel values
- **GIVEN** private names, IDs, query strings, module text or paths in the input
- **WHEN** inspection succeeds or fails
- **THEN** neither stdout nor stderr discloses those values or raw error payloads

### Requirement: Export behavior remains unchanged
Ordinary exports SHALL retain their existing parser and resolver behavior while
sharing the same raw attribute views with the diagnostic.

#### Scenario: Normal export
- **GIVEN** an ordinary export outside diagnostic execution
- **WHEN** it parses the same attributes as before
- **THEN** the shared views introduce no policy change, state leak or output change
