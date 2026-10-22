//! Native query protocols measured on 8.5.1.1592; own tiny fixture only.
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/file-infobase")
}
fn lab(tag: &str) -> (PathBuf, PathBuf) {
    // Retain failure evidence; TMPDIR is controlled by the caller.
    let root = std::env::temp_dir().join(format!(
        "native-export-{}-{tag}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let db = root.join("db");
    fs::create_dir_all(&db).unwrap();
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(fs::File::open(fixture().join("tiny.1CD.gz")).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    fs::write(db.join("1Cv8.1CD"), bytes).unwrap();
    (root, db)
}
fn run(db: &Path, words: &[&str], options: &[OsString]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ibcmd-rs"))
        .args(["infobase", "config", "export"])
        .args(words)
        .arg(format!("--db-path={}", db.display()))
        .arg("--platform=8.5.1")
        .args(options)
        .output()
        .unwrap()
}
fn opt(name: &str, value: &Path) -> OsString {
    let mut s = OsString::from(format!("--{name}="));
    s.push(value);
    s
}
fn ok(o: &Output) {
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(
        o.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
}
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
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
fn info_stdout_and_directory_match_native_bytes_without_database_changes() {
    let (root, db) = lab("info");
    let before = fs::read(db.join("1Cv8.1CD")).unwrap();
    let expected = fs::read(fixture().join("tiny-xml/ConfigDumpInfo.xml")).unwrap();
    let stdout = run(&db, &["info"], &[]);
    ok(&stdout);
    assert_eq!(
        stdout.stdout,
        expected.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&expected)
    );
    let out = root.join("info");
    fs::create_dir(&out).unwrap();
    let output = run(&db, &["info"], &[opt("out", &out)]);
    ok(&output);
    assert_eq!(fs::read(out.join("ConfigDumpInfo.xml")).unwrap(), expected);
    assert_eq!(output.stdout,"[INFO] Получение информации о состоянии конфигурации информационной базы...\n[INFO] Получение информации о состоянии конфигурации информационной базы успешно завершено.\n".as_bytes());
    assert_eq!(fs::read(db.join("1Cv8.1CD")).unwrap(), before);
}
#[test]
fn status_protocol_and_reference_changes_match_native() {
    let (root, db) = lab("status");
    let current = run(&db, &["info"], &[]);
    ok(&current);
    let xml = String::from_utf8(current.stdout).unwrap();
    let reference = root.join("base.xml");
    fs::write(&reference, &xml).unwrap();
    let same = run(&db, &["status"], &[opt("base", &reference)]);
    ok(&same);
    assert!(same.stdout.is_empty());
    let target = root.join("status.txt");
    ok(&run(
        &db,
        &["status"],
        &[opt("base", &reference), opt("out", &target)],
    ));
    assert_eq!(fs::read(&target).unwrap(), b"\xef\xbb\xbf");
    let changed = xml.replace("c57df04f818190488e0bdade6c7ca93900000000", &"0".repeat(40));
    assert_ne!(changed, xml);
    fs::write(&reference, changed).unwrap();
    for (extra, expected) in [
        (
            vec![],
            "modified: CommonModule.ФайловаяБазаПроверка.Module\n",
        ),
        (
            vec![OsString::from("--short")],
            "M: CommonModule.ФайловаяБазаПроверка.Module\n",
        ),
    ] {
        let mut args = vec![opt("base", &reference)];
        args.extend(extra);
        let output = run(&db, &["status"], &args);
        ok(&output);
        assert_eq!(output.stdout, expected.as_bytes());
    }
    fs::write(
        &reference,
        xml.replace(
            "CommonModule.ФайловаяБазаПроверка\"",
            "CommonModule.PreviousName\"",
        ),
    )
    .unwrap();
    let renamed = run(&db, &["status"], &[opt("base", &reference)]);
    ok(&renamed);
    assert_eq!(renamed.stdout, b"modified: all\n");
}
#[test]
fn objects_select_only_native_files_and_fail_before_writing_unknown_objects() {
    let (root, db) = lab("objects");
    let out = root.join("out");
    let output = run(
        &db,
        &["objects"],
        &[opt("out", &out), "CommonModule.ФайловаяБазаПроверка".into()],
    );
    ok(&output);
    let expected = tree(&fixture().join("tiny-xml"))
        .into_iter()
        .filter(|(name, _)| name.starts_with("CommonModules/"))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(tree(&out), expected);
    let absent = root.join("absent");
    let output = run(
        &db,
        &["objects"],
        &[opt("out", &absent), "CommonModule.NoSuchLabObject".into()],
    );
    assert_eq!(output.status.code(), Some(255));
    assert!(!absent.exists());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "[ERROR] Операция невозможна, при выполнении экспорта объектов конфигурации в XML обнаружены ошибки: Объект метаданных CommonModule.NoSuchLabObject не существует в конфигурации.\n"
    );
}
#[test]
fn native_base_noop_creates_no_directory_and_conflict_precedes_database_read() {
    let (root, db) = lab("base");
    let current = run(&db, &["info"], &[]);
    ok(&current);
    let base = root.join("base.xml");
    fs::write(&base, current.stdout).unwrap();
    let out = root.join("out");
    let output = run(
        &db,
        &[],
        &[opt("base", &base), out.clone().into_os_string()],
    );
    ok(&output);
    assert!(!out.exists());
    let output = run(
        &root.join("does-not-exist"),
        &[],
        &[opt("base", &base), "--sync".into(), out.into_os_string()],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        output.stderr,
        "Опции base и sync не могут быть использованы одновременно.\n".as_bytes()
    );
}
#[test]
fn malformed_status_and_invalid_file_target_match_native_exit_codes() {
    let (root, db) = lab("errors");
    let missing = run(&db, &["status"], &[]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());
    assert_eq!(
        missing.stderr,
        "Не указан путь к файлу ConfigDumpInfo.xml\n".as_bytes()
    );
    let info = run(&db, &["info"], &[]);
    ok(&info);
    let base = root.join("base.xml");
    fs::write(&base, info.stdout).unwrap();
    let target = root.join("dir");
    fs::create_dir(&target).unwrap();
    let error = run(&db, &["status"], &[opt("base", &base), opt("out", &target)]);
    assert_eq!(error.status.code(), Some(255));
    assert_eq!(error.stderr,"[ERROR] Операция невозможна, при вычислении изменений конфигурации информационной базы относительно ранее сохраненного состояния обнаружены ошибки: Ошибка приемника потока\n".as_bytes());
}

fn recorded_lab(case: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "native-export-recorded-{}-{}",
        case,
        uuid::Uuid::new_v4()
    ));
    let db = root.join("db");
    fs::create_dir_all(&db).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/native-export-commands")
        .join(case);
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(fs::File::open(fixture.join("database.1CD.gz")).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    fs::write(db.join("1Cv8.1CD"), bytes).unwrap();
    (root, db, fixture)
}
#[test]
fn real_native_change_add_delete_rename_matrix_is_byte_exact() {
    for case in ["change", "add", "delete", "rename"] {
        let (root, db, recorded) = recorded_lab(case);
        let info = run(&db, &["info"], &[]);
        ok(&info);
        let expected = fs::read(recorded.join("full/ConfigDumpInfo.xml")).unwrap();
        assert_eq!(
            info.stdout,
            expected.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&expected),
            "{case} info"
        );
        let base = fixture().join("tiny-xml/ConfigDumpInfo.xml");
        let status = run(&db, &["status"], &[opt("base", &base)]);
        ok(&status);
        assert_eq!(
            status.stdout,
            fs::read(recorded.join("status.txt")).unwrap(),
            "{case} status"
        );
        let out = root.join("base");
        let output = run(
            &db,
            &[],
            &[opt("base", &base), out.clone().into_os_string()],
        );
        if matches!(case, "delete" | "rename") {
            assert_eq!(output.status.code(), Some(255));
            assert!(!out.exists());
            assert_eq!(output.stderr,"[ERROR] Операция невозможна, при выполнении экспорта изменений конфигурации в XML обнаружены ошибки: Требуется экспортировать конфигурацию полностью.\n".as_bytes());
        } else {
            ok(&output);
            assert_eq!(tree(&out), tree(&recorded.join("base")), "{case} base");
        }
        let sync = root.join("sync");
        for (name, bytes) in tree(&fixture().join("tiny-xml")) {
            let target = sync.join(name);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, bytes).unwrap();
        }
        let before = tree(&sync);
        let output = run(&db, &[], &["--sync".into(), sync.clone().into_os_string()]);
        if matches!(case, "delete" | "rename") {
            assert_eq!(output.status.code(), Some(255));
            assert_eq!(tree(&sync), before);
        } else {
            ok(&output);
            assert_eq!(tree(&sync), tree(&recorded.join("full")), "{case} sync");
        }
    }
}
#[test]
fn native_recursion_and_configuration_selection_match_recorded_files() {
    let (root, db, recorded) = recorded_lab("owned");
    for (tag, flags) in [
        ("shallow", vec![]),
        ("recursive", vec![OsString::from("--recursive")]),
    ] {
        let out = root.join(tag);
        let mut args = vec![opt("out", &out)];
        args.extend(flags);
        args.push("Catalog.CorpusList".into());
        let output = run(&db, &["objects"], &args);
        ok(&output);
        assert_eq!(tree(&out), tree(&recorded.join(tag)), "{tag}");
    }
    let (root, db) = lab("configuration-recursion");
    let out = root.join("recursive");
    let output = run(
        &db,
        &["objects"],
        &[
            opt("out", &out),
            "--recursive".into(),
            "Configuration.Configuration".into(),
        ],
    );
    ok(&output);
    let mut expected = tree(&fixture().join("tiny-xml"));
    expected.remove("ConfigDumpInfo.xml");
    assert_eq!(tree(&out), expected);
}
#[test]
#[cfg(unix)]
fn linked_object_output_is_rejected_without_writing_outside_destination() {
    let (root, db) = lab("links");
    let out = root.join("out");
    let outside = root.join("outside");
    fs::create_dir(&out).unwrap();
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, out.join("CommonModules")).unwrap();
    let result = run(
        &db,
        &["objects"],
        &[opt("out", &out), "CommonModule.ФайловаяБазаПроверка".into()],
    );
    assert_eq!(result.status.code(), Some(255));
    assert!(tree(&outside).is_empty());
}

#[test]
fn custom_configuration_object_selection_keeps_its_existing_scope() {
    let (root, db) = lab("custom-config");
    let out = root.join("out");
    let result = Command::new(env!("CARGO_BIN_EXE_ibcmd-rs"))
        .args(["mssql-dump-config", "--file-infobase"])
        .arg(&db)
        .args([
            "--object",
            "Configuration",
            "--main-configuration",
            "--platform",
            "8.5.1",
            "--extract-metadata-xml",
            "--extract-module-text",
            "--no-binary-rows",
            "-o",
        ])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let mut actual = tree(&out);
    actual.remove("manifest.json");
    assert_eq!(
        actual,
        BTreeMap::from([(
            "Configuration.xml".to_owned(),
            fs::read(fixture().join("tiny-xml/Configuration.xml")).unwrap()
        )])
    );
}

#[test]
fn native_version_comparison_ignores_xml_header_and_requires_object_names() {
    let (root, db) = lab("header");
    let current = run(&db, &["info"], &[]);
    ok(&current);
    let xml = String::from_utf8(current.stdout).unwrap();
    let reference = root.join("older.xml");
    fs::write(
        &reference,
        xml.replace("version=\"2.21\"", "version=\"2.20\""),
    )
    .unwrap();
    let status = run(&db, &["status"], &[opt("base", &reference)]);
    ok(&status);
    assert!(status.stdout.is_empty());
    let out = root.join("out");
    let export = run(
        &db,
        &[],
        &[opt("base", &reference), out.clone().into_os_string()],
    );
    ok(&export);
    assert!(!out.exists());
    // Native 8.5.1.1592 keeps a changed row incremental even with a 2.20 reference header.
    let changed = xml
        .replace("version=\"2.21\"", "version=\"2.20\"")
        .replace("c57df04f818190488e0bdade6c7ca93900000000", &"0".repeat(40));
    fs::write(&reference, changed).unwrap();
    ok(&run(
        &db,
        &[],
        &[opt("base", &reference), out.clone().into_os_string()],
    ));
    let written = tree(&out);
    assert_eq!(written.len(), 2);
    assert_eq!(
        written["ConfigDumpInfo.xml"],
        fs::read(fixture().join("tiny-xml/ConfigDumpInfo.xml")).unwrap()
    );
    let module = "CommonModules/ФайловаяБазаПроверка/Ext/Module.bsl";
    assert_eq!(
        written[module],
        fs::read(fixture().join("tiny-xml").join(module)).unwrap()
    );
    let objects = run(&db, &["objects"], &[opt("out", &out)]);
    assert_eq!(objects.status.code(), Some(2));
    assert!(objects.stdout.is_empty());
    assert_eq!(
        objects.stderr,
        "Не указано ни одного объекта метаданных.\n".as_bytes()
    );
}

#[test]
fn empty_published_inventory_matches_native_and_malformed_nonempty_stays_closed() {
    let (root, db) = lab("empty-info");
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(fs::File::open(fixture().join("empty.1CD.gz")).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    fs::write(db.join("1Cv8.1CD"), bytes).unwrap();
    for (platform, version) in [("8.3.27", "2.20"), ("8.5.1", "2.21")] {
        let output = run(&db, &["info"], &[format!("--platform={platform}").into()]);
        ok(&output);
        let expected = fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/native-export-commands/empty-info-{version}.xml"
        )))
        .unwrap();
        assert_eq!(
            output.stdout,
            expected.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&expected)
        );
    }
    // Default platform resolution must also tolerate a genuinely empty table.
    let output = Command::new(env!("CARGO_BIN_EXE_ibcmd-rs"))
        .args(["infobase", "config", "export", "info"])
        .arg(opt("db-path", &db))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native-export-commands/empty-info-2.20.xml")
        )
        .unwrap()
        .strip_prefix(b"\xef\xbb\xbf")
        .unwrap()
    );
    let warning = format!(
        "warning: no platform is configured for {}; assuming 8.3.27 (XML 2.20). Name it with --platform, a [[database]] entry or `platform` in ibcmd-rs.toml, or IBCMD_RS_PLATFORM.\n",
        db.display()
    );
    assert_eq!(output.stderr, warning.as_bytes());
    let bad = root.join("malformed.1CD");
    fs::write(&bad, b"not a file database").unwrap();
    let output = run(&bad, &["info"], &[]);
    assert_eq!(output.status.code(), Some(255));
    assert!(output.stdout.is_empty());
}
