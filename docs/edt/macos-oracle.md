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
Its executable and `release` file are hashed. This is **partial Java provenance**,
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
