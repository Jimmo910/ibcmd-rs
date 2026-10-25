//! Real CommonModule compiler deltas through the existing recording transaction.
//! SQL metadata uses the parent recorder's framed headers, not a native writer
//! profile. Public fixture bodies are read without changing their file infobase.
use super::*;
use crate::{
    compiler::families::modules::{CommonModuleProfile, decode_common_module_blob},
    module_blob::unpack_module_blob_text,
    server::sql_import::selected::{self, prepare_module},
};
use std::{io::Read, sync::OnceLock};

const STAGE_PART_BYTES: usize = 32;
const NEW_TEXT: &[u8] = b"Procedure SelectedBridge() Export\nEndProcedure\n";
type Rows = BTreeMap<String, Vec<u8>>;

fn codec_profile() -> CommonModuleProfile {
    let registry = crate::profile_registry::load_bundled_profile_registry().unwrap();
    CommonModuleProfile::from_effective(
        registry
            .get(&ibcmd_core::artifact::ProfileId::parse("platform-8.3.27.1989").unwrap())
            .unwrap(),
    )
    .unwrap()
}

fn public_rows() -> Rows {
    static ROWS: OnceLock<Rows> = OnceLock::new();
    ROWS.get_or_init(|| {
        // No Drop/teardown: keep this newly allocated public fixture intact.
        let root =
            std::env::temp_dir().join(format!("sql-selected-transaction-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let compressed = include_bytes!("../../../tests/fixtures/file-infobase/tiny.1CD.gz");
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(compressed.as_slice())
            .read_to_end(&mut bytes)
            .unwrap();
        let path = root.join("tiny.1CD");
        std::fs::write(&path, bytes).unwrap();
        let mut db = ibcmd_filedb::FileDb::open(&path).unwrap();
        crate::mssql_dump::file_db_rows_from_reader(&mut db)
            .unwrap()
            .into_iter()
            .map(|(key, bytes)| (key, bytes.as_ref().clone()))
            .collect()
    })
    .clone()
}

struct Fixture {
    snapshot: Readset,
    source_path: String,
    body_key: String,
    old_text: Vec<u8>,
}
impl Fixture {
    fn new() -> Self {
        let config = public_rows();
        let codec = codec_profile();
        let (owner, source_path) = config
            .iter()
            .find_map(|(key, bytes)| {
                decode_common_module_blob(bytes, &codec).ok().map(|module| {
                    (
                        key.clone(),
                        format!("CommonModules/{}/Ext/Module.bsl", module.name),
                    )
                })
            })
            .expect("public tiny fixture needs one supported CommonModule");
        let body_key = format!("{owner}.0");
        let old_text = unpack_module_blob_text(&config[&body_key]).unwrap();
        assert_ne!(
            old_text.as_slice(),
            NEW_TEXT,
            "fixture must actually change"
        );
        // The effective root and inventory are valid complete staged overlays.
        // Root is an unselected multipart neighbour with different per-part
        // metadata; selected bodies must inherit their staged part-zero header.
        let saved = Rows::from([
            ("root".into(), config["root"].clone()),
            ("versions".into(), config["versions"].clone()),
            (body_key.clone(), config[&body_key].clone()),
        ]);
        let p = profile();
        let mut builder =
            Builder::new(p.identity, p.schema, Limits::default(), Arc::new(|| Ok(()))).unwrap();
        for (table, rows, staged) in [
            (Table::Config, &config, false),
            (Table::ConfigSave, &saved, true),
        ] {
            for (name, bytes) in rows {
                assert!(!bytes.is_empty(), "fixture has an empty packed member");
                let width = if !staged {
                    bytes.len()
                } else if name == "root" {
                    1
                } else {
                    17
                };
                for (number, payload) in bytes.chunks(width).enumerate() {
                    let tag = if !staged {
                        1
                    } else if name == "root" {
                        16 + u8::try_from(number % 8).unwrap()
                    } else if name == "versions" {
                        8
                    } else {
                        7
                    };
                    builder
                        .push(
                            table,
                            Input {
                                key: Key {
                                    file_name: name.clone(),
                                    part_no: u32::try_from(number).unwrap(),
                                },
                                part: Part {
                                    data_size: u64::try_from(bytes.len()).unwrap(),
                                    header: header(tag),
                                    binary: Arc::new(payload.to_vec()),
                                },
                            },
                        )
                        .unwrap();
                }
            }
        }
        Self {
            snapshot: builder.finish().unwrap(),
            source_path,
            body_key,
            old_text,
        }
    }

    fn compile(&self, text: &[u8]) -> ModuleEdit {
        let check: Check = Arc::new(|| Ok(()));
        prepare_module(
            &self.snapshot,
            &self.source_path,
            text,
            &codec_profile(),
            selected::Limits::default(),
            &check,
        )
        .unwrap()
    }
}

fn write_policy() -> WritePolicy {
    WritePolicy::new(STAGE_PART_BYTES, 4096).unwrap()
}

fn assert_no_application_dispatch(events: &[String]) {
    assert!(!events.iter().any(|event| {
        event == "temp"
            || event == "populate"
            || event.starts_with("delete:")
            || event.starts_with("insert:")
            || event == "commit"
    }));
}

#[test]
fn real_common_module_delta_commits_with_exact_unselected_physical_stage() {
    let fixture = Fixture::new();
    let original_digest = fixture.snapshot.digest().unwrap();
    let edit = fixture.compile(NEW_TEXT);
    assert_eq!(edit.readset_digest(), &original_digest);
    assert_eq!(
        edit.replacements().keys().cloned().collect::<BTreeSet<_>>(),
        BTreeSet::from([fixture.body_key.clone(), "versions".into()])
    );
    let client = Client::new(fixture.snapshot.clone(), Fault::None);
    let report = stage(
        &client,
        &fixture.snapshot,
        &edit,
        &profile(),
        &write_policy(),
        client.check(),
        &|| Ok(()),
    );
    assert_eq!(report.state, State::Committed, "{:?}", report.error);
    assert!(report.error.is_none());
    assert!(!report.cleanup_uncertain);
    assert_eq!(client.opens.load(Ordering::SeqCst), 1);
    let record = client.state.lock().unwrap();
    assert!(
        fixture
            .snapshot
            .parts(Table::Config)
            .eq(record.db.parts(Table::Config))
    );
    let unselected = |name: &str| !edit.replacements().contains_key(name);
    assert!(
        fixture
            .snapshot
            .parts(Table::ConfigSave)
            .filter(|(key, _)| unselected(&key.file_name))
            .eq(record
                .db
                .parts(Table::ConfigSave)
                .filter(|(key, _)| unselected(&key.file_name)))
    );
    assert!(
        fixture
            .snapshot
            .parts(Table::ConfigSave)
            .filter(|(key, _)| key.file_name == "root")
            .count()
            > 1
    );
    for (name, expected) in edit.replacements() {
        let captured_header = &fixture
            .snapshot
            .parts(Table::ConfigSave)
            .find(|(key, _)| key.file_name == *name && key.part_no == 0)
            .unwrap()
            .1
            .header;
        let parts = record
            .db
            .parts(Table::ConfigSave)
            .filter(|(key, _)| key.file_name == *name)
            .collect::<Vec<_>>();
        assert_eq!(
            parts.len(),
            expected.len().max(1).div_ceil(STAGE_PART_BYTES)
        );
        let mut joined = Vec::new();
        for (number, (key, part)) in parts.iter().enumerate() {
            assert_eq!(key.part_no, u32::try_from(number).unwrap());
            assert_eq!(part.data_size, u64::try_from(expected.len()).unwrap());
            assert_eq!(&part.header, captured_header);
            assert!(part.binary.len() <= STAGE_PART_BYTES);
            joined.extend_from_slice(&part.binary);
        }
        assert_eq!(&joined, expected);
        if name == &fixture.body_key {
            assert_eq!(unpack_module_blob_text(&joined).unwrap(), NEW_TEXT);
            assert_eq!(captured_header, &header(7));
        }
    }
    assert_eq!(
        record
            .events
            .iter()
            .filter(|event| *event == "commit")
            .count(),
        1
    );
    assert_eq!(
        record
            .events
            .iter()
            .rfind(|event| event.starts_with("insert:"))
            .unwrap(),
        "insert:versions"
    );
    assert_eq!(record.events.last().unwrap(), "close");
    assert_eq!(fixture.snapshot.digest().unwrap(), original_digest);
}

#[test]
fn real_common_module_noop_never_opens_a_session() {
    let fixture = Fixture::new();
    let edit = fixture.compile(&fixture.old_text);
    assert!(edit.replacements().is_empty());
    let client = Client::new(fixture.snapshot.clone(), Fault::None);
    let report = stage(
        &client,
        &fixture.snapshot,
        &edit,
        &profile(),
        &write_policy(),
        client.check(),
        &|| Ok(()),
    );
    assert_eq!(report.state, State::Unchanged);
    assert!(report.error.is_none());
    assert_eq!(client.opens.load(Ordering::SeqCst), 0);
    assert!(client.events().is_empty());
    fixture
        .snapshot
        .require_equal(&client.state.lock().unwrap().db)
        .unwrap();
}

#[derive(Clone, Copy)]
enum InterveningChange {
    Payload,
    Header,
    Addition,
    Removal,
}

fn stale_snapshot(fixture: &Fixture, change: InterveningChange) -> Readset {
    let p = profile();
    let mut builder =
        Builder::new(p.identity, p.schema, Limits::default(), Arc::new(|| Ok(()))).unwrap();
    for table in [Table::Config, Table::ConfigSave] {
        for (key, part) in fixture.snapshot.parts(table) {
            let neighbour = matches!(table, Table::ConfigSave) && key.file_name == "root";
            if neighbour && matches!(change, InterveningChange::Removal) {
                continue;
            }
            let mut part = part.clone();
            if neighbour && key.part_no == 0 {
                match change {
                    InterveningChange::Payload => Arc::make_mut(&mut part.binary)[0] ^= 1,
                    InterveningChange::Header => part.header = header(31),
                    InterveningChange::Addition | InterveningChange::Removal => {}
                }
            }
            builder
                .push(
                    table,
                    Input {
                        key: key.clone(),
                        part,
                    },
                )
                .unwrap();
        }
    }
    if matches!(change, InterveningChange::Addition) {
        let name = "00000000-0000-4000-8000-000000000909.0";
        assert!(
            !fixture
                .snapshot
                .parts(Table::ConfigSave)
                .any(|(key, _)| key.file_name == name)
        );
        builder
            .push(
                Table::ConfigSave,
                Input {
                    key: Key {
                        file_name: name.into(),
                        part_no: 0,
                    },
                    part: Part {
                        data_size: 6,
                        header: header(27),
                        binary: Arc::new(b"writer".to_vec()),
                    },
                },
            )
            .unwrap();
    }
    builder.finish().unwrap()
}

fn assert_stale_rejects_without_application_dml(change: InterveningChange) {
    let fixture = Fixture::new();
    let edit = fixture.compile(NEW_TEXT);
    // The edit is prepared first; only the connected recorder's state changes.
    let current = stale_snapshot(&fixture, change);
    assert!(fixture.snapshot.require_equal(&current).is_err());
    let client = Client::new(current.clone(), Fault::None);
    let report = stage(
        &client,
        &fixture.snapshot,
        &edit,
        &profile(),
        &write_policy(),
        client.check(),
        &|| Ok(()),
    );
    assert_eq!(report.state, State::RolledBack, "{:?}", report.error);
    assert!(report.error.is_some());
    assert!(!report.cleanup_uncertain);
    assert_eq!(client.opens.load(Ordering::SeqCst), 1);
    let events = client.events();
    assert_no_application_dispatch(&events);
    assert_eq!(
        events.iter().filter(|event| *event == "rollback").count(),
        1
    );
    assert_eq!(events.last().unwrap(), "close");
    // A stale refusal must not erase the other writer's intervening state.
    current
        .require_equal(&client.state.lock().unwrap().db)
        .unwrap();
}

#[test]
fn real_module_edit_rejects_stale_unselected_payload_before_dml() {
    assert_stale_rejects_without_application_dml(InterveningChange::Payload);
}
#[test]
fn real_module_edit_rejects_stale_unselected_header_before_dml() {
    assert_stale_rejects_without_application_dml(InterveningChange::Header);
}
#[test]
fn real_module_edit_rejects_stale_unselected_addition_before_dml() {
    assert_stale_rejects_without_application_dml(InterveningChange::Addition);
}
#[test]
fn real_module_edit_rejects_stale_unselected_removal_before_dml() {
    assert_stale_rejects_without_application_dml(InterveningChange::Removal);
}

fn assert_typed_cancellation_after_selected_event(prefix: &str) {
    let fixture = Fixture::new();
    let edit = fixture.compile(NEW_TEXT);
    let client = Client::new(fixture.snapshot.clone(), Fault::None);
    let state = client.state.clone();
    let reached = Arc::new(AtomicBool::new(false));
    let observed = reached.clone();
    let event = format!("{prefix}:{}", fixture.body_key);
    let needle = event.clone();
    let check: Check = Arc::new(move || {
        let occurred = state
            .lock()
            .unwrap()
            .events
            .iter()
            .any(|entry| entry == &needle);
        if occurred {
            observed.store(true, Ordering::SeqCst);
            Err(crate::cancel::Cancelled.into())
        } else {
            Ok(())
        }
    });
    let report = stage(
        &client,
        &fixture.snapshot,
        &edit,
        &profile(),
        &write_policy(),
        check,
        &|| Ok(()),
    );
    assert!(
        reached.load(Ordering::SeqCst),
        "cancellation point was never reached"
    );
    assert_eq!(report.state, State::RolledBack);
    assert!(crate::cancel::is_cancelled(report.error.as_ref().unwrap()));
    assert!(!report.cleanup_uncertain);
    assert_eq!(client.opens.load(Ordering::SeqCst), 1);
    let events = client.events();
    assert_eq!(events.iter().filter(|entry| *entry == &event).count(), 1);
    assert_eq!(
        events.iter().filter(|entry| *entry == "rollback").count(),
        1
    );
    assert!(!events.iter().any(|entry| entry == "commit"
        || entry == "delete:versions"
        || entry == "insert:versions"));
    if prefix == "delete" {
        assert!(
            !events
                .iter()
                .any(|entry| entry == &format!("insert:{}", fixture.body_key))
        );
    }
    assert_eq!(events.last().unwrap(), "close");
    fixture
        .snapshot
        .require_equal(&client.state.lock().unwrap().db)
        .unwrap();
}

#[test]
fn real_module_cancel_after_first_selected_delete_rolls_back() {
    assert_typed_cancellation_after_selected_event("delete");
}
#[test]
fn real_module_cancel_after_first_selected_insert_rolls_back() {
    assert_typed_cancellation_after_selected_event("insert");
}
