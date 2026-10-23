# EDT → XML native reference on macOS (#356)

The direction-specific laboratory entry point is
`scripts/edt-lab/edt_to_xml_macos.py`. It compares a converter output with native
XML exported from a **fresh file database loaded with the installed EDT export
of the same supplied EDT project**. The original project must be authentic,
complete, and contain no `.ibcmd-provenance` directory. It never accesses an
existing database and never deletes fixtures.

The harness requires physical paths under
`/Volumes/Work/Dev/ibcmd-rs-lab` for the project, candidate, copied EDT installation,
installation/schema evidence and new run. Native SDK may remain installed under
`/opt/1cv8`. It copies the Eclipse configuration and project into the new run,
uses the custom Mac CLI `-ini-file` to pin workspace/configuration and Java,
sets Java home/temp, HOME/XDG and native
`--data/--db-path/--temp` paths. Commands use a persistent advisory macOS lab lock;
coordinate the native/heavy slot with the batch planner as well, because older
harnesses do not use this lock. It does not replace the existing Windows oracle.

The capture retains raw command arguments, UTC start/duration/exit code,
stdout/stderr, full original/after project inventories, full reference/output
inventories, exact installed versions and binary hashes. It binds #358's
`installation.json` (`schema_version: 1`): live plugin inventory and selected immutable
runtime rows must match before and after (Mac launcher, INI/bootstrap/config files
and Java executable/release). Java libraries/modules are not fully inventoried;
this is partial JVM provenance, not a full runtime pin. The separately supplied #355 model
inventory is copied verbatim, bound to its declared release, preserved legacy
baseline and installed model JAR hashes; a structural model inventory does
not establish semantic compatibility. All command children run in their own
process group; timeout or interruption terminates/reaps that group before the
slot is released. Failure leaves evidence in place.

`comparison.json` retains every missing, extra and changed path, size and hash.
It contains both the strict comparison of **all** files and the existing
source-oracle content comparison excluding only `ConfigDumpInfo.xml` (native
storage generations cannot be invented by EDT). Original excluded rows remain
in the report. No XML, whitespace, namespace, module, form or resource bytes are
normalized. The native reference and EDT export are checked again after
conversion; a terminal verdict is written only after immutability checks.
Configuration import/export is separate from database activation: no APPLY or
runtime behavior is claimed.

## Current tool and legacy model are separate

The user changed the target to current EDT on 2026-10-23. Installed
**2026.2.1+3**, expected CLI **2026.2.1.3**, is the named current-version test
candidate; it must not be described as the latest release. Historical
2025.2.3 profiles remain unchanged. `--source-profile` is explicit and normally
must match the named installed release/XML version. An intentional current-tool
exercise with a legacy model requires `--compatibility-probe`; even byte equality
then emits `COMPATIBILITY_PROBE_MATCH`, **never PASS**. A profile name or launcher
version alone does not establish full 2026 model support.

Example (replace paths with genuine #358/#355 captures and a built candidate):

```sh
source /Volumes/Work/Dev/ibcmd-rs-lab/lab-env.sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/edt-lab/edt_to_xml_macos.py \
  --project "$LAB_AUTHENTIC_BSP_PROJECT" \
  --corpus-label BSP-current \
  --run /Volumes/Work/Dev/ibcmd-rs-lab/fixtures/issue356-bsp-current-001 \
  --edt "$LAB_EXTERNAL_EDT_CLI" \
  --installation "$LAB_EDT_INSTALLATION_JSON" \
  --schema-evidence "$LAB_EDT_MODEL_INVENTORY_JSON" \
  --edt-build 2026.2.1.3 \
  --source-profile edt-2025.2.3-xml-2.21 --compatibility-probe \
  --native /opt/1cv8/8.5.1.1592/ibcmd --native-build 8.5.1.1592 \
  --ours "$LAB_CANDIDATE" --runtime 8.5.1 --xml-version 2.21
```

## Evidence and outstanding acceptance

The implementation starts at `f9ebb8dd6a4b84d57c72a72d5941931e48fae73b`
without production codec changes or donor commits. #355 owns shared codec/model
changes; #358 owns installed EDT preparation. Real discrepancies must be sent
with exact path/hash rows and minimal synthetic reproductions to #355 before
codec edits.

Thirty-three synthetic controls passed on 2026-10-23. They exercise correct native
input routing, complete missing/extra/changed rows, CDI retention, source/tool/
plugin/Java/schema/reference mutation, incomplete runtime manifests, failed native import, preserved failed command bytes,
interrupted process cleanup, path/provenance guards, and compatibility verdicts.
Fixtures are retained under the external TMPDIR. Local log:
`/Volumes/Work/Dev/ibcmd-rs-lab/logs/issue356-harness-controls-r17.log`.

```sh
source /Volumes/Work/Dev/ibcmd-rs-lab/lab-env.sh
IBCMD_TEST_KEEP_FIXTURES=1 PYTHONDONTWRITEBYTECODE=1 \
  python3 tests/scripts/edt_to_xml_macos_test.py -v
```

Independent review identified and verified fixes for interrupted process cleanup,
external Eclipse/JVM configuration and rehashing the native reference. These
controls are not installed EDT or BSP acceptance. At the initial harness checkpoint no genuine BSP/current EDT/native capture
was available. The later empty smoke below does not provide a BSP match;
#356 remains open. A real empty-configuration smoke,
when available from #358, also cannot substitute for the required full BSP
comparison. Private corpus files and full private path inventories remain local;
only the reusable harness, synthetic controls and non-private conclusions belong
in this repository.

## Real current-version empty smoke (2026-10-23)

[Non-private summary](evidence/356-current-empty221-20261023.json) records the
actual run at `fixtures/issue356-current-empty221-001`. #357 supplied the genuine
EDT-native import of `empty-8.5.1.1592-en-001/native-xml`, not a generated project.
The source project has four files (2,696 bytes). We independently exported that
same project with **EDT 2026.2.1.3**, loaded the resulting XML into a new native
**8.5.1.1592** file database, then exported native XML and converted the original
project with explicit `edt-2025.2.3-xml-2.21` compatibility mode.

All eight version/export/create/import/conversion commands exited 0. Both
configuration files match byte for byte: ours has 2 files/12,369 bytes; native
has 3 files/13,009 bytes. The sole strict difference is native-only
`ConfigDumpInfo.xml`, 640 bytes, SHA-256
`b0b1c4a34b1c22ef1b080e257ce253b71be717186b797cd51668c493b9131e61`.
The result is **COMPATIBILITY_PROBE_MATCH**. No configuration-content codec
regression was observed in this empty control; #355 received the exact findings.

The candidate was borrowed read-only from #357's locked, offline Rust 1.95
`--no-default-features` build of the unchanged `f9ebb8dd` Rust tree, SHA-256
`e4ac476fad5b93723ecf2c335921efa19e61572f49956cdeddd611a53cd9ebbd`.
This PR changes Python/docs only. Project, selected installation files, plugins,
schema evidence and native reference passed after-run immutability checks.

The final combined EDT workspace log contains **three unresolved severity-4
entries**. `post-capture-diagnostics.json` records them without modifying any
original capture/verdict. Per-command workspace-log capture and explicit
validation-scope fields were then added to the reusable harness and covered by
a negative diagnostic control. The SDK errors are not suppressed or classified
as harmless. Content equality does not establish clean installed-EDT validation.

Full inventories, exact argv/exit/timing records, raw stdout/stderr and workspace
logs remain in the external run directory. No native/EDT corpus bodies are
committed. The native slot was released after the run. Full BSP comparison,
current-model semantic support and clean installed SDK acceptance remain open;
PR #506 remains Draft and #356 is not closed.

The original authentic-project `edt-export.stdout` also contains one mixed-case
`[Fatal Error] :1:1: Premature end of file.` line. It is preserved in a separate
`post-capture-stdout-diagnostics.json` supplement. The local scanner now detects
case-insensitive error/fatal markers; its regression control retains this exact
line. The same direct-project/fresh-workspace route therefore produces this
error on a genuine native-import project too; it alone does not prove a codec
regression. It is not waived as harmless.

Retained CLI and shutdown-hook logs are scanned too; fatal-only records in either
file prevent a clean diagnostic verdict even when stdout/stderr and `.log` are
empty. The negative control exercises both files.


## Nonempty public DCS/forms probe with Server JVM selection

The parent authorized a bounded nonempty probe over the public DcsFilterEvidence
native tree: 19 files, 86,112 bytes, tree
`efc5814c4b31e7c8892511003e591fe0f0567a9f0d1d69e3a2594ada60b51a72`.
#358 copied its provenance; #357 performed the single genuine installed-EDT
import. We reused that exact authentic `NativeControl` project after #357
released the shared slot, avoiding a second native-to-EDT import.

The optional `--jvm-library` pins the bound Java home's
`lib/server/libjvm.dylib`. It must appear in the installation runtime inventory,
is hashed in the private launch receipt, and is rechecked after capture. The
Java-executable selection remains available unchanged. Three additional controls
cover explicit selection, omitted inventory and library mutation. This remains
selected-runtime provenance, not a complete JVM modules/libraries inventory.

[Non-private failure summary](evidence/356-current-server-dcs-20261023.json)
records `fixtures/issue356-current-server-dcs-001`. Installed EDT export and fresh
native create/import/export all returned 0. The authentic project contains
19 files/79,140 bytes; the installed EDT export has 18 files/82,953 bytes;
the fresh native reference has 19 files/86,112 bytes. Full inventories and hashes
remain local. The candidate is the same immutable master Rust-tree binary used
in the empty control.

Our conversion decoded successfully but failed closed during encode preflight:
`conversion.edt-encode-preflight-failed`. The canonical config-interface guard
expects CRLF after the XML declaration/root of `ClientApplicationInterface.cai`;
the genuine current serializer produces LF. No output was published, so full
output equality was **not reached**. The 639-byte public sidecar reproducer is
retained in `fixtures/issue356-sidecar-repro-001`, SHA-256
`f1659827aed39cb1240e6dc9e4252c8efa66559a3c5448e7e75e96d29e52d62b`.
The exact report and reproducer were sent to #357 (agreed sidecar-fix owner) and
#355; this PR does not edit their shared codec files.

The genuine direct-project export also emitted the same mixed-case Fatal Error
and one severity-4 workspace entry. They remain in the per-command raw logs and
`diagnostic-supplement.json`, unsuppressed. All original project, installation,
schema and candidate hashes remained unchanged. The native slot was released.
An overlapping #381 compilation was reported by the coordinator, so these
command timings are not performance measurements. No further heavy phase or
retest starts until the parent grants it. Full BSP/UH acceptance and clean SDK
validation remain open; PR #506 remains Draft.


## PR review: whole-application overlap and configuration custody

The two actual Codex findings in PR #506 were confirmed against `9c601e78`.
Before any output directory, snapshot, copy or process, the harness now validates
`.app/Contents/Eclipse/1cedtcli` layout and rejects either-direction overlap
between the run and the entire physical application bundle. Existing-ancestor
`samefile` checks cover case aliases on the external case-insensitive volume.
The regression controls check configuration/Contents/MacOS/Resources descendants,
an ancestor and the real case alias without executing a recursive copy.

The entire source configuration tree is recorded in
`configuration-source-before.json`. A complete `configuration-copy-before.json`
must match before any launch. Both initial tree hashes enter the terminal
result, separately from the supplied installation-manifest hash; unlisted extra
configuration files therefore cannot silently share the same effective-input
identity. Full source/copy after-inventories and `configuration-custody.json`
retain private runtime cache changes without normalizing them away.

Configuration custody is captured before later manifest revalidation, including
when a listed configuration file changes. Missing, renamed or unreadable copied
configuration fails custody and cannot publish a verdict; a retained rename
control covers this without deletion. The source configuration must remain
unchanged. Thirty-three bounded retained-fixture controls pass; no native run,
Rust build, private-input mutation or cleanup was needed for these review fixes.
Older smoke evidence is preserved unchanged and is not retroactively described
as having this stronger custody contract.


## Corrected candidate: fast direct route and next DCS boundary

The immutable candidate built from combined production head `5e5b5bcd`, SHA-256
`6da274e498d6b21ef017c00bc8698cf6c14596d996604acce40a77bfe8431016`,
passed #357's generated-project forward/return checks. We separately tested the
**genuine installed-EDT project**, which is a different source route. The
[non-private summary](evidence/356-current-dcs-fixed-direct-20261023.json) binds
`fixtures/issue356-current-server-dcs-fixed-direct-002`.

No SDK/JVM was restarted. Before running the converter, the local capture
checked complete current/terminal genuine project inventories, complete current
installed-export inventories against the exact original native-import input,
the live native reference against its original full inventory, exact original
native create/import/export command routes and exit codes, SDK binary identity,
and the corrected candidate/build receipt. All selected raw receipts and input
trees were checked again afterward. The original native reference was reused
only because the entire installed-export input is identical, not because a
summary or file count matches. Prior configuration-custody limitations remain
visible and are not retroactively changed by this direct run.

The candidate gets past the earlier client-interface refusal but still fails
closed at encode preflight on
`Catalogs/FilterProbe/Forms/ListForm/Attributes/Список/ExtInfo/ListSettings.dcss`:
`dcss: must be CRLF, found Lf`. Decode completes, exit code is 2, and output is
not published. Full native equality and a direct three-way report are therefore
not reached. All input, reference, candidate and selected evidence hashes remain
unchanged. Minimal authentic public sidecars are retained under
`fixtures/issue356-dcss-repro-001`; exact reports and hashes were sent to #355
(shared codec owner) and #357. This PR makes no independent shared codec edit.

The retained `fixed-direct-001` attempt stopped on a capture receipt assertion
before invoking the converter; the checker was corrected to bind #357's actual
explicit-project export route and run in a fresh directory. Neither attempt
rewrote old failures. Full BSP acceptance and clean SDK validation remain open.
