# Current EDT oracle on macOS

Refs #358; shared laboratory prerequisite for #355–357. This is capture tooling,
not a completed BSP/UH oracle or a claim that the converter supports a new model.
The user's 2026-10-23 instruction changed new testing to current EDT. Existing
`edt-2025.2.3-xml-2.20` and `edt-2025.2.3-xml-2.21` profiles remain unchanged.
A test using one of those writer models and a 2026 installed tool must record
both versions separately and describe it as a compatibility probe.

## Verified baseline

On 2026-10-23, the [official download page](https://edt.1c.ru/docs/new/download.php)
advertised **2026.2.2**. The installed Mac components were
`1c-edt-2026.2.0+289-x86_64` and `1c-edt-2026.2.1+3-x86_64`. The tested local
build is **2026.2.1.3**, not the newest advertised release. No 2025 acquisition or
installation is needed for this changed target. No browser access is required.

The official installed 2026.2.1 component was copied without changing the
original installation into:

```
/Volumes/Work/Dev/ibcmd-rs-lab/tools/edt-2026.2.1+3-x86_64/
  1cedt (2026.2.1+3).app/Contents/Eclipse/1cedtcli
```

The Mac CLI is a custom launcher: it accepts `-ini-file`, `-data`, `-timeout`,
`-vmargs`, and `-command`. Generic Eclipse `-vm`, `-configuration`, `-user`,
`-nl` belong in the retained custom INI, not the CLI argument list. The harness
makes relative `../Eclipse/` bootstrap paths absolute when moving that INI. It
omits the optional original `-Dosgi.debug=` property to avoid bootstrap tracing
in the exact version output; raw command/workspace diagnostics remain intact.
The original INI is immutable and hashed. The complete copied app, including
its configuration/p2 caches, is external. A shared nonblocking advisory lock
serializes cooperating Mac EDT/native captures through final inventory checks.

The explicitly selected Java is the already installed read-only
`/Applications/1C/1CE/components/axiom-jdk-full-25.0.4+3-x86_64/bin/java`.
Its executable, `release` file and explicitly selected Server VM library are hashed.
This is **partial Java provenance**,
not a complete JDK inventory. Java `user.home`/`java.io.tmpdir`, environment
HOME/TMP/TEMP/XDG paths, workspace, user area, command logs and evidence all point
at a fresh external run. The harness removes inherited Java argument-injection
environment variables before launching. It never launches the default workspace,
opens the original ERP database, deletes fixtures, or changes security settings.

## Reproduce discovery and preparation

Source `lab-env.sh` and use a new run name; existing evidence cannot be resumed
or overwritten. No Rust build is required for this capture step.

```sh
python3 -B scripts/edt-lab/prepare_macos.py discover \
  --edt '/Volumes/Work/Dev/ibcmd-rs-lab/tools/edt-2026.2.1+3-x86_64/1cedt (2026.2.1+3).app/Contents/Eclipse/1cedtcli' \
  --java /Applications/1C/1CE/components/axiom-jdk-full-25.0.4+3-x86_64/bin/java \
  --edt-build 2026.2.1.3 \
  --jvm-library /Applications/1C/1CE/components/axiom-jdk-full-25.0.4+3-x86_64/lib/server/libjvm.dylib \
  --run /Volumes/Work/Dev/ibcmd-rs-lab/logs/issue358-current-discovery-NEW \
  --timeout 240
```

`prepare` additionally takes `--native PATH --corpus ACTUAL-LABEL
--xml-version 2.20|2.21`; it checks that Configuration.xml declares that version,
imports with the corresponding runtime 8.3.27/8.5.1 and `--build true`, then exports
using the installed serializer. Inputs must be physical external lab trees.
`PREPARED_NOT_ACCEPTED` only means commands, structural checks and immutable
input/tool checks completed. Raw error diagnostics are unresolved evidence,
never automatically waived. Preparation does not run a converter or provide any
of the three route acceptance results described in [oracle.md](oracle.md).

The schema_version 1 `installation.json` contains the complete immutable plugin
snapshot (`root`, `files[{path,bytes,sha256}]`, count, total bytes and tree SHA-256),
CLI path/hash and absolute `runtime_files` rows for the real Mac launcher,
original/custom INIs, CLI startup script, Info.plist, configuration/config.ini,
bundles.info, `.options`, Java executable and Java release. Mutable installation
configuration caches are deliberately outside the immutable payload set.
`installation-after.json` must match before any terminal capture receipt.
Command receipts retain exact argument arrays, working directory, isolated
environment overrides, start time, duration, timeout and native exit code, plus
unmodified stdout/stderr. SIGTERM/interrupt/timeout kill and reap the owned process
group. Failure evidence and all control fixtures are retained; cleanup is to
Trash by a separate user-authorized action.

## Actual evidence and remaining gates

`logs/issue358-current-discovery-003` contains actual installed-tool commands:
`version`, `help`, `help import`, `help export`, `help validate`, each exit 0.
Version stdout is exactly `2026.2.1.3`. The complete plugin inventory contains
1,870 files / 1,639,204,208 bytes with tree SHA-256
`b409df68cb99a34d81e3994cc212d8517dcc158f88488dd321f7f17f9657d09d`.
CLI SHA-256 is
`d44b5890360f382280f66035ea627b47e6ef24c2169d5fef1783578f807eab1f`.
Before/after immutable inventories are equal. Each run preserves the exact
executed harness and oracle source; later safety refinements are verified by
retained controls, not retroactively attributed to that executed source.

Earlier discovery-001 records unsupported CLI `-vm` failing before JVM startup.
Discovery-002 records a successful real version command followed by strict version
rejection because optional bootstrap tracing preceded the version. Neither has a
positive terminal discovery result. Both are retained.

The separate #355 model inventory binds installed JARs and compares legacy class
and resource inventories; it does not establish semantic schema equivalence or
justify silently relabeling the 2025 converter profiles.

The retained private native XML identifies its actual configuration as
`УправлениеПредприятием` 2.5.27.70, compatibility 8.3.27, XML 2.21. It is an ERP
corpus; it is not established as BSP or ERP UH. No private configuration bodies,
CFs, exports, full private inventories, or database files belong in GitHub.
Full current-version BSP/UH native→EDT→installed export preparation, fresh native
reference, structured validation/control and all three exact route comparisons
remain required. Empty/public fixture smoke tests can demonstrate the laboratory
pipeline but cannot close that corpus acceptance gap.

Controls: `python3 -B tests/scripts/edt_prepare_macos_test.py` with external
TMPDIR from lab-env.sh. The synthetic controls prove guards/receipts only and
never count as installed EDT compatibility or BSP/UH acceptance.

## Actual empty 2.21 compatibility smoke and JVM diagnosis

The subsequent real captures use the genuine native empty XML fixture from
`fixtures/empty-8.5.1.1592-en-001/native-xml`, not XML generated by ibcmd-rs.
#357 imported it with installed EDT to create NativeControl and exported it with
that installed serializer. Its generated provenance-free project then exported
byte equal with the same serializer; its unchanged reverse conversion matched
all original native XML files including ConfigDumpInfo.xml. The capture remains
strict **FAIL** because validation emitted `[Fatal Error] :1:1: Premature end of
file.` and workspace errors. Zero process exits and an empty TSV do not waive it.

#356 used that same authentic NativeControl project, ran the installed export,
created a fresh native 8.5.1.1592 file database, imported and re-exported it.
Its legacy-profile EDT reader output matched every configuration-content byte
of that independent native export. The sole strict inventory difference was
native ConfigDumpInfo.xml, retained in the report. This is
`COMPATIBILITY_PROBE_MATCH`, not new-model acceptance. Original projects, tool,
schema and input inventories remained unchanged.

The raw [direct reader](../evidence/edt-current-empty221/direct-edt-to-xml.json),
[installed forward serializer](../evidence/edt-current-empty221/installed-forward-serializer.json)
and [unchanged return](../evidence/edt-current-empty221/unchanged-return.json)
reports retain **all three path rows** in each route. Native tool is 8.5.1.1592,
EDT 2026.2.1.3, candidate ibcmd-rs 0.4.1 using the separately declared legacy
2025.2.3 XML-2.21 model. Exact raw classification counts are:

| Route | All equal | Native=ours only | EDT=ours only | Other disagreements |
|---|---:|---:|---:|---:|
| Direct EDT reader | 1 | 1 | 1 | 0 |
| Installed forward serializer | 1 | 0 | 2 | 0 |
| Unchanged return | 1 | 2 | 0 | 0 |

These raw serializer differences are deliberately not normalized. Route equality
criteria differ as explained in oracle.md: raw `all_equal` counts alone are not
acceptance. Reproduction uses the existing `source-three-way-oracle` command with
explicit `--native`, `--edt`, `--ours`, `--source-version 2.21`, and exact tool
versions; all three complete commands/stdout/stderr, driver source and before/after
input inventories are retained at `logs/issue358-three-way-current-empty221-001`.
Captured input trees are unchanged. Full local route evidence is at
`fixtures/issue357-current-empty221-001` and
`fixtures/issue356-current-empty221-001`. None of these are BSP/UH acceptance.

A concrete setup defect was then isolated: Eclipse selected **Client VM** even
with the supplied Axiom bin/java and `-server`. The supported INI `-vm` entry
pointing directly at the actual `lib/server/libjvm.dylib` made the real installed
EDT report **Server VM**, without changing/spoofing VM properties. The additional
library hash is in runtime_files. Version-only evidence is retained at
`logs/issue358-current-server-discovery-001`. On a copy of the authentic native
project, matched validation with that actual Server VM completed with exit 0,
empty TSV and **empty stderr**, with unchanged source/installation inventories
(`logs/issue358-server-native-validation-001`). The headless workbench lifecycle
severity-4 record still appears; it is retained and has not been waived. This
control does not retroactively turn the earlier generated validation into PASS;
the generated route must be rerun with the same corrected VM before comparing
phase-specific diagnostics.

Read-only inspection also identified the remaining workbench diagnostic's exact
origin. The actual control INI and argument array contain no
`java.awt.headless` flag. The installed
`com.e1c.g5.dt.applications.infobases.ui_1.0.1003.v202609171902.jar` (SHA-256
`5d34e50d650ea38bab84d23f8d269987463c330d8987226b8469d525f96bd785`)
contains `LifecycleAwareApplicationsViewOpener.onProjectStart`: it checks
`PlatformUI.isWorkbenchRunning`, calls the plugin's `logError` and returns when
there is no workbench. The supported command-line workflow has no UI workbench;
this evidence does not implicate a forced headless flag in our command. The
read-only javap receipt, complete raw output and exact JAR hash are retained at
`logs/issue358-headless-vendor-inspection-001`; vendor class bodies are not
published. This identifies the error source, not a waiver or evidence that all
future headless errors are benign. Export and validation command results,
complete XML equality and this error are reported separately.
