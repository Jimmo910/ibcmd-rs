# Design: diagnostic observer, first extraction boundary

## Input and lifetime

`observe_view(ViewInput<'_>, Limits, &mut dyn FnMut() -> Result<(), ObserveError>)` borrows an ordered slice of `PackedCandidate` for the synchronous call only. Each candidate supplies a view occurrence ordinal, origin/table labels, exact stored name, logical key and immutable packed bytes. `ViewInput` supplies a view label plus exact schema/platform labels. These labels remain caller assertions. This ordinal is not SQL PartNo or proof of physical part linkage; that linkage remains adapter-owned. No static/global registration, parent handles, row mutation, database access, filesystem IO, worker transport, Send/Sync or serialization contract is introduced.

The engine computes a length-framed SHA-256 over the complete ordered input, including labels, ordinals, keys and packed bytes. Old/new observations are separate calls; their labels alone do not establish chronology or mandatory artifact verification. The future adapter must bind the old view captured before edits and the new mandatory-verified artifact to their full computed map hashes. Observation never accepts a caller hash in place of computing one. The digest domain is `ibcmd-import-observation-v1`, distinct from the planner canonical packed-map hash; the adapter must retain both and bind them explicitly, never compare unlike domains. Input order is never sorted or deduplicated.

## Extraction boundary and coverage

The private metadata audit accepts bounded inflated UTF-8 text and preserves every extraction warning/miss. It normalizes adoption/header/legacy layouts before returning a header; original packed, inflated UTF-8 and normalized-text digests are distinct. Individual normalization trace remains explicitly Unavailable; audit_issue=None does not prove absence of normalization. UUID/kind/name are normalized decoder candidates, never raw owner authority. Four families only (Catalog, Document, BusinessProcess, ChartOfCharacteristicTypes) use the existing strict `decode_owner_graph` and ordered collection provenance. Preserve collection occurrence order, role, marker, schema source and each item digest. Nested serialized child UUIDs are not guessed; collection item digest is not a full parent binding.

Duplicate exact keys, stored names, ordinals and slash/case aliases are retained and flagged ambiguous; no last-writer map or UUID wildcard. The existing audit searches for a header using the supplied exact key; returned UUID is key-anchored, not independently discovered. A recognized four-family row without that header preserves its warning, is marked unbound/ambiguous and graph Rejected. Here ambiguous means no unique binding, not evidence of multiple owners. No other UUID is scanned or inferred. Header UUID/key mismatch is also ambiguous. Unknown profiles remain unsupported even if a decoder returns candidates. Registered exact platform labels mean catalog recognition only, not raw profile validation. Every result denies owner completeness. Full parent paths, body role/route binding, versions origin/count/ordinal/generation, source projection and complete inventory remain Unsupported. No `ExplicitComplete` owner dimension is exposed.

## Bounds and cancellation

Validate aggregate packed bytes, candidate count and label lengths before hashing/decode. Inflate incrementally with byte limits and cancellation checks; check normalized text/output item bounds. These are input/output limits, not a claim of preempting private decoder execution or a total allocator budget. ObserveError::Cancelled preserves typed cancellation. Cancellation/limit failure returns no partial success. No raw payload or application names appear in errors/Debug output.

## Source ownership and next hooks

Only new `src/mssql_dump/import_evidence.rs` and its module declaration change. Existing metadata, owner graph, assets, refs, projection and versions logic stay intact. A future exact pre-normalization/root/versions hook requires separate overlap review. No generic scanner/parser duplicate in SQL. No fake FileDb header. Consumer integration needs fresh review and continues unconditional authority refusal.

## Verification status

Source-only phase: synthetic positive and negative test sources will be added before integration. Cargo/native/SQL runs are not authorized in this phase and have not run. No native or production SQL acceptance is claimed.
