# Interface metadata profile — 2026-10-25

Refs #558, #544, #525.

## Independent provenance

The tool1cd repository is research documentation only; no code, serialization tree, private native output or dependency is imported. Inspected immutable revision: [7c4b91cb7a7bb48a32be00b9f32afe740dada6e1](https://github.com/jjzhang166/tool1cd/tree/7c4b91cb7a7bb48a32be00b9f32afe740dada6e1).

- [Interfaces collection identity](https://github.com/jjzhang166/tool1cd/blob/7c4b91cb7a7bb48a32be00b9f32afe740dada6e1/MetaTree.txt#L20339), [collection serialization](https://github.com/jjzhang166/tool1cd/blob/7c4b91cb7a7bb48a32be00b9f32afe740dada6e1/MetaTree.txt#L53946).
- [Interface descriptor serialization](https://github.com/jjzhang166/tool1cd/blob/7c4b91cb7a7bb48a32be00b9f32afe740dada6e1/MetaTree.txt#L55949).
- [Metadata UUID registration](https://github.com/jjzhang166/tool1cd/blob/7c4b91cb7a7bb48a32be00b9f32afe740dada6e1/MetaData.cpp#L7495).

These are hypotheses checked against native controls, not evidence of modern behavior on their own.

## Measured native descriptor

Native controls: Windows 8.3.27.2214 and 8.5.1.1150, original supplied interface set and own edits to synonyms, comments and subsystem membership. The 8.3 input archive was built on 8.3.27.2342; the actual local reader/compiler is 8.3.27.2214.

| Coordinate | Measured meaning |
| --- | --- |
| Configuration family | Interfaces / Interface, class 39bddf6a-0c3c-452b-921c-d99cfa1c2f1b |
| XML order | after CommonPicture, before SessionParameter; Bot precedes PaletteColor on 8.5.1 |
| Outer descriptor | three members: marker 1, payload, zero child-collection count |
| Payload | revision 7, inverted Switchable flag, standard metadata header, marker 1 |
| Switchable | stored 0 → XML true; stored 1 → XML false |
| Header | UUID, Name, localized Synonym, Comment; default identity/tail members must match |
| Text | a stored CR-CR-LF in an Interface comment exports as a literal XML CRLF, not a CR entity followed by LF |

A native source control containing two synonym languages, escaped XML text and a multiline comment compiled/applied/exported successfully on both platforms. A separate native source-to-CF control with a Subsystem.Content reference to Interface compiled and exported successfully on both platforms. Membership is carried by the subsystem reference: the Interface descriptor profile is unchanged, and no fictitious Subsystems property is added to its XML.

Unsupported header members, descriptor revisions, flag codes, child collections, XML properties, namespaces and ambiguous/missing required values are refused. This descriptor evidence does not establish support for arbitrary populated interface menu/body payloads or older unmeasured descriptor revisions.

## Regressions

On base 12a81c0529, the new positive tests failed with `not yet: no row decoder for Interface` and `Interface must be in root inventory` (2 failed, 0 passed).

Latest narrow checks before the final native candidate:

- `cargo test --locked -p ibcmd-rs --test interface_metadata --no-default-features`: 5 passed. Exercises both dialects and Switchable values, native multiline text, exact synthetic CF output, unknown profiles, source admission and default CLI refusal.
- `cargo test --locked -p ibcmd-rs --lib interface_root_tests --no-default-features`: 2 passed. Exercises actual production root classification, a missing/class-declared future Interface record, and refusal instead of silently dropping the root inventory.
- `npx --yes @fission-ai/openspec@1.6.0 validate add-interface-descriptors --strict --no-interactive`: passed.

Final CF/SQL/combined measurements and the complete verification gate are recorded below after execution; narrow tests alone do not establish release parity.
