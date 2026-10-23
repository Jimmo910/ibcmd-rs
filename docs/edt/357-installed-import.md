# XML → EDT installed-project capture (#357)

`scripts/edt-lab/xml_to_edt_macos.py` captures the forward route independently
of the Windows `oracle.py` orchestration. It is lab-only and launches actual
installed EDT commands, rather than treating parser/schema tests as acceptance.

The user changed the installed-tool target to current EDT on 23 October 2026.
Record the exact tested executable build separately from the writer model
profile. The converter currently exposes `edt-2025.2.3-xml-2.20` and
`edt-2025.2.3-xml-2.21`; exercising those against `2026.2.1.3` establishes a
compatibility observation for that build/corpus. It does not rename the model,
prove a 2026 schema, or establish acceptance against EDT 2025.2.3. The installed
2026.2.1+3 selected by #358 must not be described as the latest release.
A new model profile needs separate schema evidence through #355.

## Preconditions and storage

Use #358's complete external installation copy, including its p2/configuration
and plugins. A symlink to the internal app is refused. Supply the verified Java
executable explicitly. The harness passes `-vm`, a fresh explicit `-data` for
every independent installed route, JVM `user.home` and `java.io.tmpdir`, and
external HOME/TMPDIR/XDG directories. Eclipse installation/configuration caches
remain in the external copy. No user's existing workspace is opened.
[Equinox documents configuration and workspace locations separately](https://help.eclipse.org/latest/topic/org.eclipse.platform.doc.isv/reference/misc/runtime-options.html).
The complete external installation requirement matters in addition to `-data`.

The lab root must already exist under `/Volumes/`; the native source and EDT
executable must be below it. The run must be new, its parent must exist, and it
must not overlap the source. Links, traversal, special files and case collisions
are refused. Snapshot resource ceilings are harness safety bounds, not product
configuration-size support claims. No cleanup runs, even on failure. Inspect
and move unneeded runs to Trash manually. Keep source XML, generated projects,
private hashes/paths, caches and logs on the external disk; never publish private
ERP artifacts. Production converter semantics are unchanged.

## Running

After verifying the installed version and a storage-safe launch through #358,
coordinate the installed JVM run with the planner. Set the external environment:

```sh
source /Volumes/Work/Dev/ibcmd-rs-lab/lab-env.sh
export IBCMD_TEST_KEEP_FIXTURES=1 PYTHONDONTWRITEBYTECODE=1
python3 scripts/edt-lab/xml_to_edt_macos.py \
  --lab-root /Volumes/Work/Dev/ibcmd-rs-lab \
  --native /Volumes/Work/Dev/ibcmd-rs-lab/fixtures/CORPUS/native-xml \
  --run /Volumes/Work/Dev/ibcmd-rs-lab/fixtures/NEW-UNIQUE-RUN \
  --ours-exe /Volumes/Work/Dev/ibcmd-rs-lab/build/issue-357/debug/ibcmd-rs \
  --edt-exe /Volumes/Work/Dev/ibcmd-rs-lab/tools/EXTERNAL-EDT/VERIFIED-CLI \
  --edt-vm /PATH/TO/VERIFIED/JAVA \
  --edt-installation-manifest /Volumes/Work/Dev/ibcmd-rs-lab/logs/issue358-current-discovery-001/installation.json \
  --source-version 2.21 --edt-build 2026.2.1.3
```

Paths for the installation and JVM must come from the actual #358 receipt, not
from these placeholders. Repeat independently for 2.20 with its genuine source
and required runtime 8.3.27. The 2.21 route stamps runtime 8.5.1. A runtime missing
from the installed model is a failed route, never an automatic profile upgrade.

The harness first verifies the actual exact EDT version. It imports the native
XML and exports it as the authentic same-serializer control. It then converts
the original XML to EDT using the declared legacy model profile, makes a full
copy omitting only `.ibcmd-provenance`, and makes installed EDT load/export and
validate that copy in separate fresh workspaces. It copies files into that
control without deleting provenance from any tree. All command arguments,
return codes, original streams, workspace logs and complete path/size/SHA-256
manifests are retained. The #358 installation manifest is checked against every
live plugin and listed immutable runtime file before and after the run; it binds
both CLI and Java. Legacy writer schema resource hashes are recorded separately
without claiming current schema equivalence. Failed launches retain receipts too.

## Verdicts and limits

A corpus PASS requires all of:

- byte-exact installed generated XML versus the authentic installed serializer,
  with no path exclusions;
- an exact conversion back from the retained generated project to the original
  XML tree, including `ConfigDumpInfo.xml`, with no path exclusions;
- no configuration errors, malformed/unknown validation rows, command errors,
  workspace errors, or errors in CLI/shutdown logs;
- unchanged native source, generated project, provenance-free copy, installed
  exports, candidate executable, EDT executable and Java executable.

The exact XML return uses the generated project with its provenance. Installed
EDT loads the separate provenance-free copy. This does not claim byte-exact
native XML from EDT's own serializer, a provenance-free converter round trip,
or support for every configuration. An empty/public fixture PASS cannot close
#357's unrestricted real-corpus acceptance. Add genuine BSP/UH/ERP coverage and
investigate mismatches without relaxing comparisons or diagnostic gates.

`python3 tests/scripts/xml_to_edt_macos_test.py` runs synthetic negative controls,
retaining fixtures under TMPDIR. These are CI harness checks and are explicitly
not installed EDT acceptance.

## Initial local evidence

At base `f9ebb8dd`, a Rust 1.95 standalone offline build (jobs=2) and three
retained real converter round trips succeeded: genuine empty 2.20 (3 files),
genuine empty 2.21 (3 files), and public subsystem 2.21 (6 files). Every path and
byte matched and each source stayed unchanged. Local evidence is in
`fixtures/issue357-offline-001/summary.json` beneath the external lab root.
These results do not launch EDT and do not establish installed acceptance.
Current-version installed results are recorded separately after #358 setup.
