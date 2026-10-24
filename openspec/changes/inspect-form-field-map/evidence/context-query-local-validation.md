# Optional context/query observer: local validation

The optional mode binds the compressed Form body and both frozen XML byte streams
by SHA256. A reviewed caller supplies VM-local exact attribute-name/MainTable
context. Rust checks custody, unique raw-attribute correspondence and paired
MainTable equality; it does not independently attest how the caller extracted
that context from XML. Context strings are excluded from report and Debug output.

The final-SELECT parser retains repeated plain paths only in diagnostic mode.
The ordinary parser uses the same implementation with projection disabled; its
paths/aliases/stars/source results and consumers remain unchanged. A public
equivalence control compares both modes after removing diagnostic-only fields.
No production field-resolution or export policy is changed.

## Passed local public gates

Rust 1.95.0 on macOS ARM64, jobs=1, external target/temp, IBCMD_TEST_KEEP_FIXTURES=1:

| Gate                                                                                | Result               |
| ----------------------------------------------------------------------------------- | -------------------- |
| `cargo test --locked --lib field_map_observer --no-default-features`                | 15/15 PASS,0 ignored |
| `cargo test --locked --lib query_identity_observer --no-default-features`           | 11/11 PASS,0 ignored |
| `cargo test --locked --lib form_observer_shared_schema_tests --no-default-features` | 1/1 PASS,0 ignored   |
| `cargo build --locked --no-default-features --example form_field_map_observer`      | PASS                 |
| Legacy public CLI controls                                                          | 11/11 PASS           |
| Context public CLI controls                                                         | 12/12 PASS           |

Fixtures and captured stdout/stderr are retained. CLI checks cover exact reordered
joins, all three wrong digests, duplicate/missing names and ordinals, conflicting
MainTable, malformed JSON, byte limits and extra arguments. No observer filesystem
mutation or private sentinel egress was observed. No private input, VM, SQL or
native operation was used.

The first local context run passed 14/15 and exposed a test-oracle false positive:
forbidden substring `attribute_name` also matched the fixed public key
`exact_attribute_name_join`. Its log and source pins remain retained. The repaired
test rejects exact raw context keys recursively, retains private-value/XML checks,
and proves private-value and nested-key poison cases fail. Reverse substitution
reconstructed the exact original file digest, proving every byte outside that
test replacement unchanged. Independent source review cleared the custody fix,
projection and oracle repair before the successful bounded retry.

Source manifest 003 SHA256:
`956d70025e4133ce758ce611d4047cea43ed44a3a7985997feb5f19ed1b4771a`.
Retained release receipt SHA256:
`b46325c5b1d833f9a1acb7596854365715e771c2d6986a27f86f6f73bfd1a4b5`.
Local macOS executable: 4,364,200 bytes, SHA256
`83880345036aab0f29e5401bb08b9d2150f4a95c7adb119249be5e680000d391`.
This executable came from the reviewed uncommitted manifest; it is not a Windows
artifact and must not be substituted for clean exact-head CI provenance.

## Remaining gates

A new exact-head Windows artifact, immutable worker/argument binding, public
Windows context/custody controls, sole-operator lease and bounded real-input facts
remain pending. The VM-local context adapter has source-only review; its 21 public
assertions and Windows sharing/ABI controls have not run. Raw context, body and
XML stay inside the VM. Earlier R8/R9 XML-only observations do not provide the
missing query identity or automatic-field provenance.

No native parity, production cause or correction for the remaining UT differences
is claimed. Default chart selection and Form marker behavior remain unresolved.
