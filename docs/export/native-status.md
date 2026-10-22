# Native version status API (#362, reused by #380)

The editor `StoredConfiguration::status` compares source bytes. Native status
compares ConfigDumpInfo versions; equal XML bytes do not imply equal versions.
Native import can restamp Configuration and Language without changing XML.

- `StoredConfiguration::native_config_dump_info(&mut self) -> Result<Vec<u8>>`
  reads the main configuration through the shared export inventory writer and
  the existing SQL handle (or own rows directory), without writing files or
  converting the object-body batches. A completed ConfigSave is included.
- `mssql_dump::read_config_dump_info(&MssqlDumpConfigArgs)` is the same reader
  for direct filedb/SQL/rows-dir callers. Set `main_configuration` for native
  main-state semantics. Selected rows, base/sync, or partial inventory are refused.
- `native_export_status::ConfigDumpInfoSnapshot::parse(&[u8])` reads the XML.
  `current.compare(&reference)` returns `NativeExportStatus` with `fullRequired`
  and changes (`name`, `id`, `kind`, `configVersion`, `referenceVersion`).
  A removed ID or renamed reference requires a full export. Each ordinary
  changed row retains its exact current and reference versions.
- `NativeExportStatus::text(short)` supplies the measured native stdout form;
  RPC should use the typed fields, retain reference state explicitly, and
  surface `fullRequired` rather than converting it to an unchanged preview.

Measured on native 8.5.1.1592: unchanged status is empty; individual rows are
`added` / `modified`; removed/renamed reference reports `modified: all`.
A reference-only XML header change 2.20 -> 2.21 does not count as a native
version change. This differs from the custom export dialect migration policy.
