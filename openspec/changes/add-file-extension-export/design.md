# Design

The physical registry layout and CAS root selection must be measured on synthetic 8.3.27.2214 file databases before implementation. SQL field offsets are not evidence for the file layout. The reader validates the complete supported profile and bounds allocations. Unknown prepared states are refused rather than silently exporting an active image.

The command reads under the file backend's read lock, uses only the selected extension manifest and referenced digests, obtains borrowed-object base context from the same database, and renders with the existing extension exporter. Output is staged; incomplete/opaque results fail before publication. Native tools are acceptance oracles only.

Risks: platform profile drift, ambiguous names, mixed CAS namespaces, unrecognized prepared state, borrowed-object context and CDI version provenance. Acceptance evidence must name exact native and Rust binaries and record unchanged database hashes.

## Measured initial profile

The supported profile is documented in evidence/physical-profile-20261023.md. It exports the applied image only when CONFIGCASSAVE and CONFIGSAVE are empty. Prepared snapshots are refused explicitly. The file envelope discriminator stays separate from the SQL decoder; no writer or SQL activation changes are included.

