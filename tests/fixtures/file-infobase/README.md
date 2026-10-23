# File infobase fixtures (#372–#375)

Written by 1C:Enterprise 8.5.1.1150 (`ibcmd`, Linux x86_64) on 2026-10-21;
the `.1CD` files are gzip-compressed (`gzip -9 -n`).

| File | How it was made |
|------|-----------------|
| `empty.1CD.gz` | `ibcmd infobase create --db-path=empty --create-database` — an infobase with no configuration (`CONFIG` holds no row). |
| `ru-empty.1CD.gz` | `ibcmd infobase create --db-path=ru-empty --locale=ru_RU --create-database` (run with `LANG=ru_RU.UTF-8`): the `ru_RU` collation. |
| `de-empty.1CD.gz` | `ibcmd infobase create --db-path=de-empty --locale=de_DE --create-database`: a collation ibcmd-rs does not write. |
| `tiny-source/` | The XML of `empty` (`ibcmd infobase config export`) with one common module added by hand (`ФайловаяБазаПроверка`, own uuid and text). |
| `tiny.1CD.gz` | `ibcmd infobase create --db-path=tiny --create-database --import=tiny-source --apply --force`. |
| `tiny.cf` | 8.5.1.1529 (Windows): `ibcmd infobase config save --db-path=<tiny> tiny.cf` of an infobase made from `tiny-source` as `tiny.1CD.gz` is. |
| `tiny-xml/` | `ibcmd infobase config export --db-path=tiny tiny-xml` — what the export of `tiny` must reproduce byte for byte. |

The format evidence these files back is in
`openspec/changes/add-file-infobase-backend/evidence/1cd-format.md`.
