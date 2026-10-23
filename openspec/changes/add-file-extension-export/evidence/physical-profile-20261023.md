# File extension physical profile, 2026-10-23

Native utility `/opt/1cv8/8.3.27.2214/ibcmd --version` returned `8.3.27.2214` on macOS. Every operation used a new synthetic database and dedicated lab data/temp directories. Production code never invokes it.

Measured `_EXTENSIONSINFO` descriptor fields, in declaration order:

| Name | Kind | Length | Collation |
| --- | --- | --- | --- |
| _IDRREF | B | 16 | CS |
| _EXTENSIONORDER | N | 9 | CS |
| _EXTNAME | NVC | 255 | CI |
| _UPDATETIME | DT | 0 | CS |
| _EXTENSIONUSEPURPOSE | N | 2 | CS |
| _EXTENSIONSCOPE | N | 2 | CS |
| _EXTENSIONZIPPEDINFO | I | 0 | CS |
| _MASTERNODE | NT | 0 | CI |
| _USEDINDISTRIBUTEDINFOBASE | L | 0 | CS |
| _VERSION | RV | 0 | CS |

All fields were non-nullable with precision zero. Index name: `_IDRREFIDX`. Records were 578 bytes. The physical RV occupies the first 16 bytes after the free/live flag; all other fields follow declaration order. A declaration-order decoder produced invalid names/blob references; RV-first decoding recovered both exact native extension names and valid blobs.

The envelope magic is `43 c2 9a 14`, followed by a 20-byte active SHA-1 root. Its body starts `a2 9a 08`, whereas the existing SQL decoder accepts `a1 9a 08`. These profiles remain distinct. Native apply reported root `0d9b84547835f35b85e0289f45616f8618ca41c3`, exactly matching the registry and a CONFIGCAS row name. The second synthetic extension used a different root `3d99fb8542039bb258a28a4b0b30bb6b8693e304`. CONFIGCAS uses the existing CONFIG-family physical layout; its logical names are digest addresses, not metadata names.

Before apply, native load populated `CONFIGCASSAVE` with namespaced logical rows plus configinfo. The initial supported route deliberately refuses nonempty CONFIGCASSAVE or CONFIGSAVE; it does not substitute an active image for an unrecognized prepared state. This is a documented limitation pending measured prepared-image support.

Two native exports succeeded separately. An unknown name returned code 1 without creating output. The database SHA-256 before/after all three exports was identical: `f69074d092c97149f534fb2aefd318c0f6d5d3a5fec7324dd9ce39528fb356da`. These are native fixture observations, not Rust parity acceptance yet.

Local raw descriptors/records and command logs are retained outside the repository in the dedicated issue520 lab run. No native XML trees, raw blobs or database files are included in this evidence document.
