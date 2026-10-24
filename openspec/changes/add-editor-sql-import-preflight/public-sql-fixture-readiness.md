# Shared public SQL probe fixture readiness

Status: source inspection only, no target bound and no SQL/native/VM operation.
This packet separates a candidate public seed from native SQL provenance. It is
not a valid operator plan or an accepted profile. The #380 read-only RPC packet
and #382 profile capture may share one source only after the sole operator proves
and freezes the exact new disposable SQL fixture under an assigned lease.

## Known source inputs

| Input | Source-inspected evidence | Permitted conclusion |
| --- | --- | --- |
| Bootstrap manifest | `tests/fixtures/bootstrap/manifest.json`, SHA-256 `07ca63b632e6b9915fcc809400c49ca83601a9678c49bcadf4aba44abe661f3a` | Hand-authored clean-room 8.3.27.1989/XCF2.20 common-module seed design. No native SQL creation/apply proof. |
| Primary snapshot batch | `src/server/sql_import/snapshot.sql`, SHA-256 `ae96e854c44ee78ff796586de10bdeafb24698be302bcad8a76a7f70647c6ac5` | Fixed read-only acquisition source, no accepted measured SQL profile. |
| Operator capture batch | `scripts/sql-lab/snapshot-profile/capture.sql`, SHA-256 `57d42f13bf4996c4723a48d2876a0b4610dff5b5698871428a5be84328897772` | Reviewed source-only metadata/header/part capture package, not a live receipt. |
| Plan validator | `scripts/sql-lab/snapshot-profile/validate.py`, SHA-256 `0fbadd8b0a6eecb482078fb5d6a833f00dd869d05e34aabaac81f48748def3b3` | Strict offline structure validation; accepted_profile stays false. |
| Public chart candidate | Public source/evidence inspected at `ut-export-diff` commit `761e7ca4c79b779e9ccb9e9001648b5ece50c1f5`; exact bindings below | Source-owned seed with candidate-compiled CF and recorded FILE observations only. Not SQL create/apply/export evidence. |
| Public-chart source controls | Retained public control receipts explicitly state `windows_or_existing_baseline_executed=false` | Pure immutable-capture/growth control evidence only; not the chart native baseline. |

The public 8.5 tiny file-infobase fixture does not prove 8.3 SQL compatibility.
Private application corpora and existing user databases are excluded. No target
is guessed from old editor connection labels or prior application measurements.

## Public chart seed bindings (source-inspected)

The fixture owner is #381; the source worktree is `repos/ut-export-diff`. Input is
ONLY `tests/fixtures/ut-chart-policy/wrapper-prototype/tree/`: all five file
names, lengths and SHA-256 values match its manifest on read-only inspection.
The manifest SHA-256 is
`aff4c4f4f7d84cb08866bdb0e9190b5bac8a4b26c6d0d477c4f2a676ac7c27c4`.
Its historical `UNMEASURED_NOT_NATIVE_ACCEPTED` status remains unchanged.

Later public evidence is
`tests/fixtures/ut-chart-policy/explicit-english-zero-series/evidence.json`,
SHA-256 `f58e71b1c68a95d3cc3d0ecff8a5862ac28b1f7d7df2fb015f97b1230983ca9c`:

- Candidate-compiled CF: 7505 bytes, SHA-256
  `b2fa2d388face5e7281711d3714815cc304e12bd220f7495be15abfa0a613b8b`.
- Recorded native build: 8.3.27.2342; executable SHA-256
  `38f37715535cea530e58d5ba2e7e072c184b61b12f3e903ac290b299cf39fd38`.
- Referenced retained native receipt SHA-256:
  `f65179de7a49cdb7acad0225a1a3853bb0215b70a3404221e6db26f1e7b3c493`.

The CF and candidate-only receipts were independently hashed at their retained
external lab paths (relative to `/Volumes/Work/Dev/ibcmd-rs-lab`):

| Artifact | Retained path | SHA-256 |
| --- | --- | --- |
| CF, 7505 bytes | `fixtures/ut-chart-wrapper-basefree-internalinfo-admission-a1d7357862ff4658a6d4a00bdfecf08f/prototype.cf` | `b2fa2d388face5e7281711d3714815cc304e12bd220f7495be15abfa0a613b8b` |
| Candidate bootstrap receipt | Same directory, `receipt.json` | `72784cf79d837e85c5508ae31e492d53abf18cb1e69858d6d6cb5901a8f5e7aa` |
| Candidate readback receipt | `fixtures/ut-chart-wrapper-readback-48a5f728eb2b4c15a39de6f91073647c/receipt.json` | `dd2ce56d4678a5cd511c4672e7f29112163d22c38fe8ac56c114a49034dd16e9` |
| Public bundle, 13564 bytes | `tmp/ut-chart-wrapper-candidate-public-003.tar.gz` | `34739b89df57ac4e7af6a2fc8879084d358cd59abe606c9ac2a66f4792409b9c` |

Both inspected candidate receipts explicitly record `native_runs=0` and
`input_is_not_native_oracle=true`. Their successful bootstrap/readback is not a
native receipt. The actual native FILE receipt custody path is requested from
#381; the executable and native result have not been independently consumed or
re-executed here. The
evidence explicitly records `native_full_acceptance=false` and incomplete final
accounting. The CF is candidate-compiled public source, not native expected
output. A FILE import/export observation is not a native SQL apply baseline.
The shorthand combining CF prefix and 7505-byte length is not a full hash.

## Missing operator-owned bindings (STOP)

- Actual retained native FILE receipt custody and current operator proof of
  the chosen seed; a known CF hash/candidate readback is not that proof.
- Fresh disposable SQL fixture native create/import/apply/export receipts on the
  actual requested platform, including native executable hash and full exported
  tree hash. A file-infobase receipt cannot fill this field.
- Independently recorded exact server/database, database GUID, family GUID,
  recovery fork, SQL ProductVersion and actual storage schema/header/part layout.
- Exact current tested candidate binary/embedded revision and matching RPC packet
  provenance; source-only observer tests are not an accepted SQL profile.
- Source freeze/exclusivity receipt and window, precise baseline hashes, and
  current sole-operator lease after the preceding operator phase releases.
- Proof of the supplied dedicated non-pooled connection/provider capabilities.

No concrete SQL target or credential is recorded in this packet. Authentication
reference is a mechanism only: the sole operator supplies an already-open,
dedicated NON-POOLED `System.Data.Common.DbConnection` using their existing
integrated/token/secure authentication mechanism. No credential or connection
string is read, copied into a plan, printed or supplied as a CLI argument. The
package changes no login, privileges, network, encryption or trust settings.

## Exact first offline action

After the operator obtains the missing independently verified bindings, populate
a fresh external private `plan.json` using the strict source-only README schema,
verify the reviewed package hashes, then run:

```text
python validate.py --plan EXTERNAL_PRIVATE_RUN/plan.json
```

Do not manufacture placeholders to make validation pass. Without those bindings,
stop before SQL dispatch. Offline plan validation does not grant a VM/SQL/native
lease or authorize database creation. After the sole operator separately obtains
the lease and disposable-target authority, the package permits one parameterized
ExecuteReader against its supplied existing dedicated connection. No USE,
reconnect, discovery fallback, or automatic provisioning is part of capture.
Actual resultset ordinals, provider cancellation, lock/coherence and source/native
preservation remain unmeasured acceptance gates.
