# БСП 3.2 demo on file infobases, 1C:Enterprise 8.5.1.1150 — 21.10.2026

Linux x86_64, 4 cores. Every infobase is a copy made for the run; the
platform's results are its own `ibcmd infobase config apply|export`.
`bsp` is `infobase create --load=1Cv8_demo.cf --apply` (9 938 `CONFIG` rows,
172 MB); `bsp-imp` is `bsp` after a native `config import` of its own export
(a completed stage in `CONFIGSAVE`); `empty` is `infobase create`.

## The writer alone (`ibcmd-filedb`)

The `CONFIGSAVE` rows of a native import, written by `ibcmd-filedb` into
another infobase, then applied and exported by the platform:

| target | rows written | `config apply` | export vs the БСП tree |
|---|---|---|---|
| `bsp-imp` (its own rows again) | 9 939 | success | identical but `ConfigDumpInfo.xml` |
| `bsp` (freed space reused) | 9 939 | success | identical but `ConfigDumpInfo.xml` |
| `empty` (objects grown, fat level 0 → 1) | 9 939 | success | identical but `ConfigDumpInfo.xml` |
| `empty`, the native `*.new` stage | 9 934 | success | the empty configuration, as the platform leaves it from the same rows |

`ConfigDumpInfo.xml` differs as it does after the platform's own import: its
generation ids are renewed (the native control `bsp-imp` differs from the
`bsp` export in that file alone). Each write took about one second. The
index of the `CONFIG` table the platform wrote during `config apply` reads
back key for key through `ibcmd-filedb`.

Two layout rules came out of failed applies (a heap corruption in the
platform, SIGSEGV): name fields padded with spaces, and `left` limited to the
previous key's significant bytes; leaves are written half full. See
`1cd-format.md`.

## `infobase config export --db-path`

| infobase | files | identical to the native export |
|---|---|---|
| `bsp` | 12 336 | 12 336 |
| `bsp-imp` (the staged configuration) | 12 336 | 12 334 |

Before the change to `ibcmd-xml` for unused data parameters that are user
settings (see the proposal), `bsp` was 12 335 of 12 336. The two `bsp-imp`
files are the chart series name of two templates (`Pivot` stored, `Сводная`
written): the native export writes that default name in the language of the
process, ibcmd-rs writes what is stored. The rows came from a native import
run in an English process; the same rows in SQL Server export the same way.
The comparisons above ran the native export with `LANG=ru_RU.UTF-8`.

7.4 s against 10.9 s for the native export of `bsp`.

## `infobase config import --db-path`

ibcmd-rs imports, then the platform applies and exports:

| case | stage | `config apply` | export vs the tree |
|---|---|---|---|
| БСП tree into `empty` | base-free | success | identical but `ConfigDumpInfo.xml` |
| БСП tree into `bsp` | patch, no change | success | identical |
| edited БСП tree (a module and a synonym) into `bsp` | patch | success | identical but `ConfigDumpInfo.xml` |
| the empty configuration into `bsp` | base-free, 9 938 rows deleted | success | identical but `ConfigDumpInfo.xml` |

22 s (into `empty`) and 32 s (into `bsp`, with the `cf load` re-export) for
ibcmd-rs, the stage guard included; 17 s for the native import into `empty`.

## Locking

- An import into an infobase a standalone server (`ibsrv`) has open is
  refused before anything is compiled; the file, its directory and the
  backup are untouched.
- An export from that open infobase succeeds (12 336 files).
- While a process holds the write lock ibcmd-rs takes for an import, the
  platform's `config export` fails with `Database shared access error`.
