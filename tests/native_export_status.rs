//! Shared native version contract for CLI and RPC consumers.
use ibcmd_rs::native_export_status::{ChangeKind, ConfigDumpInfoSnapshot};
fn tiny() -> Vec<u8> {
    include_bytes!("fixtures/file-infobase/tiny-xml/ConfigDumpInfo.xml").to_vec()
}
#[test]
fn versions_are_observed_even_when_source_xml_was_not_changed() {
    let baseline = tiny();
    let old = ConfigDumpInfoSnapshot::parse(&baseline).unwrap();
    let text = String::from_utf8(baseline).unwrap();
    // No source-file input is involved: native import can restamp these rows
    // while retaining exactly the same Configuration/Language XML bytes.
    let changed = text
        .replace("a86111168911c14b90b4fc704b63ebe600000000", &"0".repeat(40))
        .replace("446fd6627b646645af71333b00c13f6f00000000", &"1".repeat(40));
    assert_ne!(changed, text);
    let current = ConfigDumpInfoSnapshot::parse(changed.as_bytes()).unwrap();
    let result = current.compare(&old);
    assert!(!result.full_required);
    assert_eq!(result.changes.len(), 2);
    assert!(
        result
            .changes
            .iter()
            .all(|entry| entry.kind == ChangeKind::Modified)
    );
    assert_eq!(result.changes[0].name, "Configuration.Configuration");
    assert_eq!(result.changes[1].name, "Language.English");
    assert!(
        result
            .changes
            .iter()
            .all(|entry| entry.reference_version.as_deref() != Some(entry.config_version.as_str()))
    );
}
#[test]
fn native_header_only_difference_is_not_a_version_change() {
    let bytes = tiny();
    let current = ConfigDumpInfoSnapshot::parse(&bytes).unwrap();
    let older = String::from_utf8(bytes)
        .unwrap()
        .replace("version=\"2.21\"", "version=\"2.20\"");
    let status = current.compare(&ConfigDumpInfoSnapshot::parse(older.as_bytes()).unwrap());
    assert!(!status.full_required);
    assert!(status.changes.is_empty());
    assert_eq!(status.text(false), "");
}
#[test]
fn rename_or_deleted_reference_requires_full_export() {
    let bytes = tiny();
    let current = ConfigDumpInfoSnapshot::parse(&bytes).unwrap();
    let renamed = String::from_utf8(bytes).unwrap().replace(
        "CommonModule.ФайловаяБазаПроверка\"",
        "CommonModule.PreviousName\"",
    );
    let status = current.compare(&ConfigDumpInfoSnapshot::parse(renamed.as_bytes()).unwrap());
    assert!(status.full_required);
    assert_eq!(status.text(false), "modified: all\n");
    assert_eq!(status.text(true), "M: all\n");
}
#[test]
fn malformed_inventory_is_rejected() {
    assert!(ConfigDumpInfoSnapshot::parse(b"<ConfigDumpInfo/>").is_err());
    assert!(
        ConfigDumpInfoSnapshot::parse(b"<ConfigDumpInfo><Anything/></ConfigDumpInfo>").is_err()
    );
    assert!(
        ConfigDumpInfoSnapshot::parse(
            b"<ConfigDumpInfo><ConfigVersions/><ConfigVersions/></ConfigDumpInfo>"
        )
        .is_err()
    );
    assert!(ConfigDumpInfoSnapshot::parse(b"not XML").is_err());
    assert!(ConfigDumpInfoSnapshot::parse(b"<ConfigDumpInfo><ConfigVersions><Metadata name=\"X\"/></ConfigVersions></ConfigDumpInfo>").is_err());
}

#[test]
fn inventory_reader_refuses_diagnostic_writes_before_reading_any_source() {
    use clap::Parser;
    use ibcmd_rs::cli::{Cli, Commands};
    let root = std::env::temp_dir().join(format!("inventory-no-write-{}", uuid::Uuid::new_v4()));
    let journal = root.join("journal.json");
    let cli = Cli::try_parse_from([
        std::ffi::OsString::from("ibcmd-rs"),
        "mssql-dump-config".into(),
        "--rows-dir".into(),
        root.join("absent-rows").into_os_string(),
        "--runtime-journal".into(),
        journal.clone().into_os_string(),
        "--extract-metadata-xml".into(),
        "--extract-module-text".into(),
        "--no-binary-rows".into(),
        "-o".into(),
        root.join("out").into_os_string(),
    ])
    .unwrap();
    let Commands::MssqlDumpConfig(args) = cli.command else {
        panic!("dump args expected")
    };
    let error = ibcmd_rs::mssql_dump::read_config_dump_info(&args).unwrap_err();
    assert!(error.to_string().contains("runtime_journal"));
    assert!(!root.exists());
}
