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
sets explicit workspace/configuration, Java home/temp, HOME/XDG and native
`--data/--db-path/--temp` paths. Commands use a persistent advisory macOS lab lock;
coordinate the native/heavy slot with the batch planner as well, because older
harnesses do not use this lock. It does not replace the existing Windows oracle.

The capture retains raw command arguments, UTC start/duration/exit code,
stdout/stderr, full original/after project inventories, full reference/output
inventories, exact installed versions and binary hashes. It binds #358's
`installation.json` (`schema_version: 1`): live plugin inventory and immutable
runtime rows must match before and after. The separately supplied #355 model
inventory is copied verbatim and hash-bound; a structural model inventory does
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

Fourteen synthetic controls passed on 2026-10-23. They exercise correct native
input routing, complete missing/extra/changed rows, CDI retention, source/tool/
plugin/reference mutation, failed native import, preserved failed command bytes,
interrupted process cleanup, path/provenance guards, and compatibility verdicts.
Fixtures are retained under the external TMPDIR. Local log:
`/Volumes/Work/Dev/ibcmd-rs-lab/logs/issue356-harness-controls-r3.log`.

```sh
source /Volumes/Work/Dev/ibcmd-rs-lab/lab-env.sh
IBCMD_TEST_KEEP_FIXTURES=1 PYTHONDONTWRITEBYTECODE=1 \
  python3 tests/scripts/edt_to_xml_macos_test.py -v
```

Independent review identified and verified fixes for interrupted process cleanup,
external Eclipse/JVM configuration and rehashing the native reference. These
controls are not installed EDT or BSP acceptance. At this checkpoint no genuine
BSP project/current EDT/native capture is available to this owner, so there is
no BSP match claim and #356 remains open. A real empty-configuration smoke,
when available from #358, also cannot substitute for the required full BSP
comparison. Private corpus files and full private path inventories remain local;
only the reusable harness, synthetic controls and non-private conclusions belong
in this repository.
