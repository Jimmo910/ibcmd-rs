# Current EDT reader evidence

Issue #355 now targets current EDT validation while retaining the explicit
2025.2.3 profiles and their existing schema evidence. A tested executable build
and a codec model profile are separate coordinates. Neither running the legacy
writer in a newer executable nor matching class names proves current schema
support. Do not relabel the legacy profiles.

`scripts/edt-lab/model_inventory.py` inspects installed JARs without executing
EDT/JVM, extracting archives, changing the installation, or creating workspaces.
It compares every bundle and each model/importer/exporter class in the bundled
legacy inventory. Missing or ambiguous bundles remain explicit. It records JAR
hashes before and after inspection, rejects changing inputs, and records hashes
of every `.ecore`, `.xcore`, and `.genmodel` resource in the inspected bundles.
The output is created exclusively; earlier evidence is never replaced. Output
must be below an existing external `/Volumes` lab with no symlink ancestors.
Model resources are streamed with a 256 MiB diagnostic cap and manifests are
bounded to a 64 KiB main section (signed per-entry digest tails are not read); exceeding these limits fails the diagnostic rather than
truncating evidence. The archive is inspected and hashed through the same open
file descriptor, with final pathname and descriptor identity checks.

Use exact paths under the external laboratory for the output and any shell
logs. Disable Python bytecode caching so imports do not create local caches:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/edt-lab/model_inventory.py \
  --plugins "$EDT_PLUGINS" \
  --baseline crates/ibcmd-schema/data/edt-2025.2.3-model-inventory.json \
  --installed-release 2026.2.1+3 \
  --lab-root "$LAB" \
  --output "$LAB/logs/edt-2026.2.1-model-inventory.json"
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s scripts/edt-lab -p test_model_inventory.py -v
```

The release argument is a declared coordinate, not a verified executable
version or a claim to be the latest official release. Bind actual CLI version
and executable identity separately using #358's native capture.

On the Mac's installed **2026.2.1+3**, the initial read-only comparison inspected
all **76** legacy bundles and hashed **64** model resources. Two legacy model
class names are absent: `ObjectOnSupport` and `impl.ObjectOnSupportImpl` in
`com._1c.g5.v8.dt.compare.model`. No other legacy model/importer/exporter class
names were absent. New classes, unrelated/new bundles, method bodies, feature
semantics, serialization defaults and ordering are outside this comparison.
Resource hashes permit later exact comparisons; they do not establish that
legacy and current resources match.

This diagnostic is not a reader/form fix or full configuration acceptance.
The historical 95 form differences still require a current-master paired
BSP/UH census. Authentic EDT import/export and native comparisons remain
separate gates. Current-build empty-project smoke evidence cannot establish
whole BSP/UH support, arbitrary-configuration support, or legacy exact-version
acceptance.
