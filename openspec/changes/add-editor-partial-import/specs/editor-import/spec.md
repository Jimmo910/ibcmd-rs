# Contract

- Settings `[[database]].db-path` and RPC `initialize.infobases[].dbPath` designate a real file infobase (directory or data file). Settings paths resolve relative to the settings file. Responses use `source: "file-db"` and `dbPath`. Existing MSSQL entries remain compatible; rows-dir stays read-only. Reject conflicting source selectors, server+file selectors and masked file entries before connecting.
- A selected import must never feed a sparse source tree to the whole-tree importer. Under an exclusive lock, read CONFIG and CONFIGSAVE, reject incomplete stages, construct the effective configuration, export a complete baseline, overlay only approved existing selected paths, compile and verify the complete expected result, then atomically publish CONFIGSAVE.
- Preserve unselected effective objects and previous staged changes. Reject new/deleted/unsupported changes before database publication. Preserve lock, CAS, verified-copy and backup protections. Cancellation is checked before publication.
- Preflight resolves object/file selections, validates references against the effective target plus the selected set, and suggests missing source dependencies. Adding dependencies requires explicit confirmation. Binding/connection identity and target/source fingerprints must still match at commit.
- Client shows selected import and changed import actions, reports errors, confirms dependencies, and never spawns the native platform. Apply UI belongs to #384. Native acceptance orchestration uses new lab databases only.

## Scenarios

1. Invalid/conflicting connection selectors fail before opening a target or writing files.
2. Existing module/form selection stages only approved changes; unrelated objects and earlier staging survive.
3. Unsupported metadata additions, unresolved references, stale selection, cancellation, interrupted stage and CAS mismatch leave the target byte-identical.
4. Native apply and native export on a synthetic document fixture reproduce the expected selected edits and unchanged neighbour.
