//! Staged editor export acceptance. Fixtures are retained: no destructive teardown.
use ibcmd_rs::stored_objects::{ConfigSource, StoredConfiguration};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/file-infobase")
        .join(name)
}
fn setup() -> (PathBuf, StoredConfiguration) {
    let root = std::env::temp_dir().join(format!("editor-export-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(root.join("db")).unwrap();
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(fs::File::open(fixture("tiny.1CD.gz")).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    let db = root.join("db/1Cv8.1CD");
    fs::write(&db, bytes).unwrap();
    fs::create_dir(root.join("source")).unwrap();
    fs::create_dir(root.join("stage")).unwrap();
    let config = StoredConfiguration::new(
        ConfigSource::FileDb(db),
        ibcmd_rs::platform::parse("8.5.1").unwrap(),
    );
    (root, config)
}
#[test]
fn selective_stage_matches_full_native_bytes_and_never_writes_source() {
    let (root, mut config) = setup();
    let source = root.join("source");
    let stage = root.join("stage");
    let info = fs::read(fixture("tiny-xml/ConfigDumpInfo.xml")).unwrap();
    fs::write(source.join("ConfigDumpInfo.xml"), &info).unwrap();
    fs::write(source.join("README.md"), b"unrelated").unwrap();
    let name = "CommonModule.ФайловаяБазаПроверка".to_owned();
    let plan = config
        .prepare_editor_export(&[name.clone()], false, &source, &stage, &|_, _, _| {})
        .unwrap();
    assert_eq!(plan.objects, vec![name]);
    assert!(plan.files.iter().any(|n| n.ends_with("Module.bsl")));
    for path in plan.files {
        assert_eq!(
            fs::read(stage.join(&path)).unwrap(),
            fs::read(fixture("tiny-xml").join(path)).unwrap()
        );
    }
    assert_eq!(fs::read(source.join("ConfigDumpInfo.xml")).unwrap(), info);
    assert_eq!(fs::read(source.join("README.md")).unwrap(), b"unrelated");
    assert_eq!(fs::read_dir(&source).unwrap().count(), 2);
}
#[test]
fn changed_is_native_version_based_and_preserves_noop_inventory() {
    let (root, mut config) = setup();
    let source = root.join("source");
    let stage = root.join("stage");
    let info = fs::read(fixture("tiny-xml/ConfigDumpInfo.xml")).unwrap();
    fs::write(source.join("ConfigDumpInfo.xml"), &info).unwrap();
    let plan = config
        .prepare_editor_export(&[], true, &source, &stage, &|_, _, _| {})
        .unwrap();
    assert!(plan.objects.is_empty());
    assert!(plan.files.is_empty());
    assert!(plan.removed.is_empty());
    let changed = String::from_utf8(info)
        .unwrap()
        .replace("a86111168911c14b90b4fc704b63ebe600000000", &"0".repeat(40));
    fs::write(source.join("ConfigDumpInfo.xml"), changed).unwrap();
    let plan = config
        .prepare_editor_export(&[], true, &source, &stage, &|_, _, _| {})
        .unwrap();
    assert!(!plan.objects.is_empty());
    assert_eq!(
        fs::read(stage.join("ConfigDumpInfo.xml")).unwrap(),
        fs::read(fixture("tiny-xml/ConfigDumpInfo.xml")).unwrap()
    );
}
#[test]
fn deleted_descriptor_and_assets_are_planned_but_never_deleted() {
    let (root, mut config) = setup();
    let source = root.join("source");
    let stage = root.join("stage");
    let mut info = ibcmd_xml::config_dump_info::ConfigDumpInfo::parse(
        &fs::read(fixture("tiny-xml/ConfigDumpInfo.xml")).unwrap(),
    )
    .unwrap();
    info.entries.push(ibcmd_xml::config_dump_info::Entry {
        name: "CommonModule.Deleted".into(),
        id: "00000000-0000-0000-0000-000000000999".into(),
        config_version: "gone".into(),
        children: vec![],
    });
    fs::write(
        source.join("ConfigDumpInfo.xml"),
        info.to_native_bytes().unwrap(),
    )
    .unwrap();
    fs::create_dir_all(source.join("CommonModules/Deleted/Ext")).unwrap();
    fs::write(source.join("CommonModules/Deleted.xml"), b"old").unwrap();
    fs::write(
        source.join("CommonModules/Deleted/Ext/Module.bsl"),
        b"old module",
    )
    .unwrap();
    fs::write(
        source.join("CommonModules/Deleted/.ibcmd-note"),
        b"unrelated note",
    )
    .unwrap();
    let plan = config
        .prepare_editor_export(&[], true, &source, &stage, &|_, _, _| {})
        .unwrap();
    assert!(!plan.removed.iter().any(|p| p.contains(".ibcmd-note")));
    assert!(plan.removed.contains(&"CommonModules/Deleted.xml".into()));
    assert!(
        plan.removed
            .contains(&"CommonModules/Deleted/Ext/Module.bsl".into())
    );
    assert!(source.join("CommonModules/Deleted.xml").exists());
    assert_eq!(
        fs::read(stage.join("ConfigDumpInfo.xml")).unwrap(),
        fs::read(fixture("tiny-xml/ConfigDumpInfo.xml")).unwrap()
    );
}
#[test]
fn overlapping_stage_is_rejected_before_writing() {
    let (root, mut config) = setup();
    let source = root.join("source");
    let stage = source.join("nested");
    fs::create_dir(&stage).unwrap();
    assert!(
        config
            .prepare_editor_export(
                &["Configuration".into()],
                false,
                &source,
                &stage,
                &|_, _, _| {}
            )
            .is_err()
    );
    assert_eq!(fs::read_dir(stage).unwrap().count(), 0);
}

fn tree(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            (
                e.path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}
#[test]
fn changed_add_modify_delete_and_rename_reconcile_to_native_full_inventory() {
    for case in ["add", "change", "delete", "rename"] {
        let (root, _) = setup();
        let source = root.join("source");
        let stage = root.join("stage");
        let native = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/native-export-commands")
            .join(case);
        let mut db = Vec::new();
        flate2::read::GzDecoder::new(fs::File::open(native.join("database.1CD.gz")).unwrap())
            .read_to_end(&mut db)
            .unwrap();
        fs::write(root.join("db/1Cv8.1CD"), db).unwrap();
        let mut files = tree(&fixture("tiny-xml"));
        for (name, bytes) in &files {
            let path = source.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        fs::write(source.join("README.md"), b"unrelated").unwrap();
        let mut config = StoredConfiguration::new(
            ConfigSource::FileDb(root.join("db/1Cv8.1CD")),
            ibcmd_rs::platform::parse("8.5.1").unwrap(),
        );
        let plan = config
            .prepare_editor_export(&[], true, &source, &stage, &|_, _, _| {})
            .unwrap();
        for name in &plan.removed {
            files.remove(name);
        }
        for name in &plan.files {
            files.insert(name.clone(), fs::read(stage.join(name)).unwrap());
        }
        assert_eq!(files, tree(&native.join("full")), "case {case}");
        assert_eq!(fs::read(source.join("README.md")).unwrap(), b"unrelated");
    }
}
#[test]
fn group_and_multiselection_produce_same_object_bytes_and_inventory() {
    for selectors in [
        vec!["Configuration/CommonModules".to_owned()],
        vec![
            "CommonModule.ФайловаяБазаПроверка".to_owned(),
            "CommonModule.ФайловаяБазаПроверка".to_owned(),
        ],
    ] {
        let (root, mut config) = setup();
        let source = root.join("source");
        let stage = root.join("stage");
        fs::write(
            source.join("ConfigDumpInfo.xml"),
            fs::read(fixture("tiny-xml/ConfigDumpInfo.xml")).unwrap(),
        )
        .unwrap();
        let plan = config
            .prepare_editor_export(&selectors, false, &source, &stage, &|_, _, _| {})
            .unwrap();
        assert_eq!(plan.objects, vec!["CommonModule.ФайловаяБазаПроверка"]);
        for file in plan.files {
            assert_eq!(
                fs::read(stage.join(&file)).unwrap(),
                fs::read(fixture("tiny-xml").join(&file)).unwrap(),
                "{file}"
            );
        }
    }
}
#[test]
fn partial_inventory_keeps_unselected_version_and_never_advertises_it_current() {
    let (root, mut config) = setup();
    let source = root.join("source");
    let stage = root.join("stage");
    let mut info = ibcmd_xml::config_dump_info::ConfigDumpInfo::parse(
        &fs::read(fixture("tiny-xml/ConfigDumpInfo.xml")).unwrap(),
    )
    .unwrap();
    let language = info
        .entries
        .iter_mut()
        .find(|e| e.name == "Language.English")
        .unwrap();
    language.config_version = "old-language".into();
    fs::write(
        source.join("ConfigDumpInfo.xml"),
        info.to_native_bytes().unwrap(),
    )
    .unwrap();
    config
        .prepare_editor_export(
            &["CommonModule.ФайловаяБазаПроверка".into()],
            false,
            &source,
            &stage,
            &|_, _, _| {},
        )
        .unwrap();
    let after = ibcmd_xml::config_dump_info::ConfigDumpInfo::parse(
        &fs::read(stage.join("ConfigDumpInfo.xml")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        after
            .entries
            .iter()
            .find(|e| e.name == "Language.English")
            .unwrap()
            .config_version,
        "old-language"
    );
}
