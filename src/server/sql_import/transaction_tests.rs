use super::super::{
    readset::{Builder, Limits},
    selected::recording_edit,
};
use super::*;
use crate::{
    mssql_live_continue::DatabaseIdentity,
    sql::{ScriptVariables, SqlRow, SqlSession, SqlValue},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
const BODY: &str = "00000000-0000-4000-8000-000000000101.0";
const NEIGHBOUR: &str = "00000000-0000-4000-8000-000000000102.0";
fn profile() -> Profile {
    Profile {
        identity: DatabaseIdentity {
            server: "recording".into(),
            database: "public_synthetic".into(),
            database_guid: uuid::Uuid::from_u128(1).to_string(),
            family_guid: uuid::Uuid::from_u128(2).to_string(),
            recovery_fork: uuid::Uuid::from_u128(3).to_string(),
        },
        sql_version: "recording-only".into(),
        schema: [11; 32],
    }
}
fn header(tag: u8) -> Vec<u8> {
    let mut h = Vec::new();
    for b in [tag, tag + 1, tag + 2] {
        h.extend_from_slice(&1u64.to_le_bytes());
        h.push(b);
    }
    h
}
fn original() -> Readset {
    let p = profile();
    let mut b = Builder::new(p.identity, p.schema, Limits::default(), Arc::new(|| Ok(()))).unwrap();
    for (table, name, no, size, data, tag) in [
        (Table::Config, BODY, 0, 6, b"abc".as_slice(), 1),
        (Table::Config, BODY, 1, 6, b"def", 1),
        (Table::Config, "versions", 0, 3, b"old", 1),
        (Table::Config, NEIGHBOUR, 0, 3, b"raw", 1),
        (Table::ConfigSave, BODY, 0, 6, b"old", 7),
        (Table::ConfigSave, BODY, 1, 6, b"old", 7),
        (Table::ConfigSave, "versions", 0, 2, b"sv", 7),
        (Table::ConfigSave, NEIGHBOUR, 0, 5, b"stage", 9),
    ] {
        b.push(
            table,
            Input {
                key: Key {
                    file_name: name.into(),
                    part_no: no,
                },
                part: Part {
                    data_size: size,
                    header: header(tag),
                    binary: Arc::new(data.to_vec()),
                },
            },
        )
        .unwrap();
    }
    b.finish().unwrap()
}
fn edit(original: &Readset) -> ModuleEdit {
    recording_edit(
        original,
        BTreeMap::from([
            (BODY.into(), b"XY".to_vec()),
            ("versions".into(), b"newV".to_vec()),
        ]),
    )
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Fault {
    #[default]
    None,
    StaleHeader,
    Identity,
    Schema,
    BadSeal,
    Uncommittable,
    ZeroSession,
    OtherSession,
    PostBytes,
    MissingHeader,
    DeleteCount,
    InsertCount,
    ReadFailure,
    CommitLost,
    RollbackFailure,
    CancelTemp,
    CancelPost,
    LateCancel,
}
struct Record {
    db: Readset,
    events: Vec<String>,
    fault: Fault,
    cancel: Arc<AtomicBool>,
}
struct Client {
    state: Arc<Mutex<Record>>,
    opens: AtomicUsize,
}
impl Client {
    fn new(db: Readset, fault: Fault) -> Self {
        Self {
            state: Arc::new(Mutex::new(Record {
                db,
                events: vec![],
                fault,
                cancel: Arc::new(AtomicBool::new(false)),
            })),
            opens: AtomicUsize::new(0),
        }
    }
    fn check(&self) -> Check {
        let cancel = self.state.lock().unwrap().cancel.clone();
        Arc::new(move || {
            if cancel.load(Ordering::SeqCst) {
                anyhow::bail!("recorded-cancel");
            }
            Ok(())
        })
    }
    fn events(&self) -> Vec<String> {
        self.state.lock().unwrap().events.clone()
    }
}
struct Session {
    state: Arc<Mutex<Record>>,
    working: Readset,
    temp: Vec<Input>,
    deleted: BTreeSet<String>,
    reads: usize,
    begun: bool,
    poisoned: bool,
    terminal: bool,
}
fn str_param<'a>(p: &'a [SqlParam<'a>], n: usize) -> &'a str {
    match p[n] {
        SqlParam::Text(v) => v,
        _ => panic!("text parameter"),
    }
}
fn int_param(p: &[SqlParam<'_>], n: usize) -> i64 {
    match p[n] {
        SqlParam::I64(v) => v,
        _ => panic!("integer parameter"),
    }
}
impl Session {
    fn start(&mut self) -> Result<()> {
        ensure!(
            !self.poisoned && !self.terminal,
            "recorded unusable session"
        );
        self.poisoned = true;
        Ok(())
    }
    fn frame_rows(&self, params: &[SqlParam<'_>]) -> Vec<SqlRow> {
        let p = profile();
        let fault = self.state.lock().unwrap().fault;
        let mut frames = vec![SqlRow {
            result_set: 0,
            values: vec![
                SqlValue::Text(if fault == Fault::Identity {
                    "changed".into()
                } else {
                    p.identity.server
                }),
                SqlValue::Text(p.identity.database),
                SqlValue::Text(p.identity.database_guid),
                SqlValue::Text(p.identity.family_guid),
                SqlValue::Text(p.identity.recovery_fork),
                SqlValue::Text(p.sql_version),
                SqlValue::Binary(if fault == Fault::Schema {
                    vec![12; 32]
                } else {
                    p.schema.to_vec()
                }),
            ],
        }];
        let mut counts = [0i64; 2];
        let mut sizes = [0i64; 2];
        for (index, table) in [(0, Table::Config), (1, Table::ConfigSave)] {
            for (key, part) in self.working.parts(table) {
                counts[index] += 1;
                sizes[index] += part.binary.len() as i64;
                let mut offset = 0;
                let mut fields = Vec::new();
                for _ in 0..3 {
                    let len =
                        u64::from_le_bytes(part.header[offset..offset + 8].try_into().unwrap())
                            as usize;
                    offset += 8;
                    fields.push(part.header[offset..offset + len].to_vec());
                    offset += len;
                }
                assert_eq!(offset, part.header.len());
                if fault == Fault::StaleHeader
                    && self.reads == 1
                    && index == 1
                    && key.file_name == NEIGHBOUR
                {
                    fields[0][0] ^= 1;
                }
                let mut data = part.binary.as_ref().clone();
                if fault == Fault::PostBytes
                    && self.reads == 2
                    && index == 1
                    && key.file_name == NEIGHBOUR
                {
                    data[0] ^= 1;
                }
                frames.push(SqlRow {
                    result_set: index + 1,
                    values: vec![
                        SqlValue::Text(key.file_name.clone()),
                        SqlValue::Int(i64::from(key.part_no)),
                        SqlValue::Int(part.data_size as i64),
                        SqlValue::Binary(fields[0].clone()),
                        SqlValue::Binary(fields[1].clone()),
                        SqlValue::Binary(fields[2].clone()),
                        SqlValue::Binary(data),
                    ],
                });
            }
        }
        frames.push(SqlRow {
            result_set: 3,
            values: vec![
                SqlValue::Text(str_param(params, 6).into()),
                SqlValue::Int(counts[0]),
                SqlValue::Int(counts[1]),
                SqlValue::Int(sizes[0]),
                SqlValue::Int(sizes[1]),
                SqlValue::Int(if fault == Fault::BadSeal { 0 } else { 1 }),
                SqlValue::Int(if fault == Fault::Uncommittable { -1 } else { 1 }),
                SqlValue::Int(if fault == Fault::ZeroSession {
                    0
                } else if fault == Fault::OtherSession && self.reads == 2 {
                    86
                } else {
                    85
                }),
            ],
        });
        frames
    }
}
impl SqlSession for Session {
    fn run_batch(&mut self, sql: &str) -> Result<()> {
        self.start()?;
        let fault = self.state.lock().unwrap().fault;
        if sql.starts_with("-- ibcmd-editor:begin") {
            ensure!(!self.begun, "already begun");
            assert!(
                sql.find("USE [master]").unwrap() < sql.find("USE [public_synthetic]").unwrap()
            );
            assert!(sql.contains("sp_getapplock") && !sql.contains("sp_releaseapplock"));
            assert!(sql.contains("@LockOwner='Session'"));
            assert!(sql.contains("backupset") && sql.contains("57269"));
            self.begun = true;
            self.state.lock().unwrap().events.push("begin".into());
        } else if sql == TEMP {
            ensure!(self.begun, "temp before transaction");
            self.state.lock().unwrap().events.push("temp".into());
        } else if sql == COMMIT {
            ensure!(self.begun, "commit without transaction");
            let mut record = self.state.lock().unwrap();
            record.events.push("commit".into());
            record.db = self.working.clone();
            self.begun = false;
            if fault == Fault::CommitLost {
                anyhow::bail!("commit response lost");
            }
            if fault == Fault::LateCancel {
                record.cancel.store(true, Ordering::SeqCst);
            }
        } else {
            anyhow::bail!("unexpected session batch");
        }
        self.poisoned = false;
        Ok(())
    }
    fn execute(&mut self, sql: &str, params: &[SqlParam<'_>]) -> Result<u64> {
        self.start()?;
        ensure!(self.begun, "DML without transaction");
        let fault = self.state.lock().unwrap().fault;
        let name = str_param(params, 0).to_string();
        assert!(sql.contains("CONVERT(varbinary(max),CONVERT(nvarchar(128),"));
        if sql.starts_with("-- ibcmd-editor:populate") {
            assert_eq!(params.len(), 4);
            assert!(sql.contains("@@ROWCOUNT<>1"));
            if fault == Fault::MissingHeader {
                anyhow::bail!("metadata provenance is not unique");
            }
            let metadata = self
                .working
                .parts(Table::ConfigSave)
                .chain(self.working.parts(Table::Config))
                .find(|(key, _)| key.file_name == name && key.part_no == 0)
                .unwrap()
                .1
                .header
                .clone();
            let bytes = match params[2] {
                SqlParam::Binary(v) => v.to_vec(),
                _ => panic!("payload"),
            };
            self.temp.push(Input {
                key: Key {
                    file_name: name,
                    part_no: int_param(params, 3) as u32,
                },
                part: Part {
                    data_size: int_param(params, 1) as u64,
                    header: metadata,
                    binary: Arc::new(bytes),
                },
            });
            let mut r = self.state.lock().unwrap();
            r.events.push("populate".into());
            if fault == Fault::CancelTemp {
                r.cancel.store(true, Ordering::SeqCst);
            }
        } else if sql.starts_with("-- ibcmd-editor:delete") {
            assert_eq!(params.len(), 2);
            assert!(
                sql.contains("DELETE s FROM dbo.ConfigSave s WHERE")
                    && sql.contains("@@ROWCOUNT<>@P2")
            );
            self.state
                .lock()
                .unwrap()
                .events
                .push(format!("delete:{name}"));
            let count = self
                .working
                .parts(Table::ConfigSave)
                .filter(|(key, _)| key.file_name == name)
                .count();
            ensure!(
                count as i64 == int_param(params, 1) && fault != Fault::DeleteCount,
                "delete count changed"
            );
            self.deleted.insert(name);
        } else if sql.starts_with("-- ibcmd-editor:insert") {
            assert_eq!(params.len(), 2);
            assert!(sql.contains("@@ROWCOUNT<>@P2"));
            self.state
                .lock()
                .unwrap()
                .events
                .push(format!("insert:{name}"));
            ensure!(
                self.deleted.contains(&name),
                "insert before selected delete"
            );
            let parts = self
                .temp
                .iter()
                .filter(|input| input.key.file_name == name)
                .map(|input| Input {
                    key: input.key.clone(),
                    part: input.part.clone(),
                })
                .collect::<Vec<_>>();
            ensure!(
                parts.len() as i64 == int_param(params, 1) && fault != Fault::InsertCount,
                "insert count changed"
            );
            self.working = self.working.with_selected_stage(parts)?;
        } else {
            anyhow::bail!("unexpected statement/pool writer");
        }
        self.poisoned = false;
        Ok(0) // NOCOUNT means this driver count is NOT an admission proof.
    }
    fn read_rows(
        &mut self,
        sql: &str,
        params: &[SqlParam<'_>],
        each: &mut dyn FnMut(SqlRow) -> Result<()>,
    ) -> Result<()> {
        self.start()?;
        ensure!(self.begun, "read outside held transaction");
        assert!(!sql.contains("USE __DATABASE__") && !sql.contains("USE ["));
        let config = sql
            .find("FROM dbo.Config WITH (TABLOCKX,HOLDLOCK)")
            .unwrap();
        let saved = sql
            .find("FROM dbo.ConfigSave WITH (TABLOCKX,HOLDLOCK)")
            .unwrap();
        assert!(config < saved);
        assert!(!sql.contains(" ROLLBACK TRANSACTION;\n SELECT @P7"));
        self.reads += 1;
        self.state.lock().unwrap().events.extend([
            "lock-config".into(),
            "lock-configsave".into(),
            format!("read:{}", self.reads),
        ]);
        if self.state.lock().unwrap().fault == Fault::ReadFailure {
            anyhow::bail!("read transport failed");
        }
        for row in self.frame_rows(params) {
            each(row)?;
        }
        let r = self.state.lock().unwrap();
        if r.fault == Fault::CancelPost && self.reads == 2 {
            r.cancel.store(true, Ordering::SeqCst);
        }
        self.poisoned = false;
        Ok(())
    }
    fn rollback_only(&mut self) -> Result<()> {
        self.state.lock().unwrap().events.push("rollback".into());
        ensure!(
            !self.poisoned && self.begun && !self.terminal,
            "rollback protocol/transaction unusable"
        );
        self.terminal = true;
        ensure!(
            self.state.lock().unwrap().fault != Fault::RollbackFailure,
            "rollback acknowledgement lost"
        );
        self.begun = false;
        Ok(())
    }
}
impl SqlClient for Client {
    fn dbms(&self) -> Dbms {
        Dbms::SqlServer
    }
    fn max_connections(&self) -> usize {
        1
    }
    fn with_dedicated_session(
        &self,
        work: &mut dyn FnMut(&mut dyn SqlSession) -> Result<()>,
    ) -> Result<()> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        let working = self.state.lock().unwrap().db.clone();
        let mut session = Session {
            state: self.state.clone(),
            working,
            temp: vec![],
            deleted: BTreeSet::new(),
            reads: 0,
            begun: false,
            poisoned: false,
            terminal: false,
        };
        let result = work(&mut session);
        self.state.lock().unwrap().events.push("close".into());
        if result.is_ok() && session.poisoned {
            anyhow::bail!("swallowed request failure");
        }
        if self.state.lock().unwrap().fault == Fault::LateCancel {
            anyhow::bail!("outer late cancellation");
        }
        result
    }
    fn run_script(&self, _: &str, _: ScriptVariables) -> Result<()> {
        panic!("pooled/script fallback")
    }
    fn execute(&self, _: &str, _: &[SqlParam<'_>]) -> Result<u64> {
        panic!("pooled execute")
    }
    fn read_rows(
        &self,
        _: &str,
        _: &[SqlParam<'_>],
        _: &mut dyn FnMut(SqlRow) -> Result<()>,
    ) -> Result<()> {
        panic!("pooled read")
    }
    fn query_json(&self, _: &str) -> Result<Option<String>> {
        panic!("pooled JSON")
    }
    fn write_rows(&self, _: &str, _: &[&str], _: &[Vec<SqlParam<'_>>]) -> Result<u64> {
        panic!("multi-session bulk writer")
    }
}
fn run(fault: Fault) -> (Report, Client, Readset) {
    let original = original();
    let client = Client::new(original.clone(), fault);
    let report = stage(
        &client,
        &original,
        &edit(&original),
        &profile(),
        &WritePolicy::new(2, 4096).unwrap(),
        client.check(),
        &|| Ok(()),
    );
    (report, client, original)
}
#[test]
fn commit_is_one_session_preserves_neighbours_and_shrinks_selected_tail() {
    let (report, client, original) = run(Fault::None);
    assert_eq!(report.state, State::Committed, "{:?}", report.error);
    assert_eq!(client.opens.load(Ordering::SeqCst), 1);
    let r = client.state.lock().unwrap();
    assert!(original.parts(Table::Config).eq(r.db.parts(Table::Config)));
    assert!(
        original
            .parts(Table::ConfigSave)
            .filter(|(k, _)| k.file_name == NEIGHBOUR)
            .eq(r
                .db
                .parts(Table::ConfigSave)
                .filter(|(k, _)| k.file_name == NEIGHBOUR))
    );
    let selected =
        r.db.parts(Table::ConfigSave)
            .filter(|(k, _)| k.file_name == BODY)
            .collect::<Vec<_>>();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].1.binary.as_slice(), b"XY");
    assert_eq!(selected[0].1.header, header(7));
    assert_eq!(
        r.events.iter().filter(|v| v.as_str() == "commit").count(),
        1
    );
    let last_insert = r
        .events
        .iter()
        .rposition(|v| v.starts_with("insert:"))
        .unwrap();
    assert_eq!(r.events[last_insert], "insert:versions");
    assert_eq!(r.events.last().unwrap(), "close");
}
#[test]
fn stale_full_state_and_invalid_identity_schema_or_seal_dispatch_no_dml() {
    for fault in [
        Fault::StaleHeader,
        Fault::Identity,
        Fault::Schema,
        Fault::BadSeal,
        Fault::Uncommittable,
        Fault::ZeroSession,
        Fault::ReadFailure,
    ] {
        let (report, client, _) = run(fault);
        assert_ne!(report.state, State::Committed);
        assert!(report.error.is_some());
        assert!(
            !client
                .events()
                .iter()
                .any(|v| v.starts_with("delete:") || v.starts_with("insert:") || v == "temp")
        );
        assert_eq!(client.opens.load(Ordering::SeqCst), 1);
    }
}
#[test]
fn post_write_mismatch_or_session_change_rolls_back_without_commit() {
    for fault in [Fault::PostBytes, Fault::OtherSession, Fault::CancelPost] {
        let (report, client, original) = run(fault);
        assert_eq!(report.state, State::RolledBack, "{:?}", report.error);
        assert!(!client.events().iter().any(|v| v == "commit"));
        original
            .require_equal(&client.state.lock().unwrap().db)
            .unwrap();
    }
}
#[test]
fn metadata_and_rowcount_failures_never_commit_and_keep_uncertainty() {
    for fault in [Fault::MissingHeader, Fault::DeleteCount, Fault::InsertCount] {
        let (report, client, original) = run(fault);
        assert_eq!(report.state, State::Unknown);
        assert!(report.cleanup_uncertain);
        assert!(!client.events().iter().any(|v| v == "commit"));
        original
            .require_equal(&client.state.lock().unwrap().db)
            .unwrap();
    }
}
#[test]
fn cancellation_at_temp_rolls_back_before_any_application_write() {
    let (report, client, original) = run(Fault::CancelTemp);
    assert_eq!(report.state, State::RolledBack);
    assert!(
        !client
            .events()
            .iter()
            .any(|v| v.starts_with("delete:") || v == "commit")
    );
    original
        .require_equal(&client.state.lock().unwrap().db)
        .unwrap();
}
#[test]
fn commit_response_loss_is_unknown_without_rollback_or_retry() {
    let (report, client, original) = run(Fault::CommitLost);
    assert_eq!(report.state, State::Unknown);
    assert_eq!(client.opens.load(Ordering::SeqCst), 1);
    assert!(!client.events().iter().any(|v| v == "rollback"));
    assert!(
        original
            .require_equal(&client.state.lock().unwrap().db)
            .is_err()
    );
}
#[test]
fn late_cancel_cannot_revoke_acknowledged_commit() {
    let (report, client, _) = run(Fault::LateCancel);
    assert_eq!(report.state, State::Committed);
    assert!(report.error.is_some());
    assert_eq!(client.opens.load(Ordering::SeqCst), 1);
}
#[test]
fn source_guard_and_budget_fail_before_open_and_noop_never_opens() {
    let original = original();
    let client = Client::new(original.clone(), Fault::None);
    let policy = WritePolicy::new(2, 4096).unwrap();
    let r = stage(
        &client,
        &original,
        &edit(&original),
        &profile(),
        &policy,
        client.check(),
        &|| anyhow::bail!("source changed"),
    );
    assert_eq!(r.state, State::NotPublished);
    assert_eq!(client.opens.load(Ordering::SeqCst), 0);
    let r = stage(
        &client,
        &original,
        &edit(&original),
        &profile(),
        &WritePolicy::new(2, 1).unwrap(),
        client.check(),
        &|| Ok(()),
    );
    assert_eq!(r.state, State::NotPublished);
    assert_eq!(client.opens.load(Ordering::SeqCst), 0);
    let empty = recording_edit(&original, BTreeMap::new());
    let r = stage(
        &client,
        &original,
        &empty,
        &profile(),
        &policy,
        client.check(),
        &|| Ok(()),
    );
    assert_eq!(r.state, State::Unchanged);
    assert_eq!(client.opens.load(Ordering::SeqCst), 0);
}
#[test]
fn rollback_failure_is_unknown_and_never_retried() {
    let original = original();
    let client = Client::new(original.clone(), Fault::RollbackFailure);
    let calls = AtomicUsize::new(0);
    let guard = || {
        if calls.fetch_add(1, Ordering::SeqCst) >= 2 {
            anyhow::bail!("final source changed");
        }
        Ok(())
    };
    let r = stage(
        &client,
        &original,
        &edit(&original),
        &profile(),
        &WritePolicy::new(2, 4096).unwrap(),
        client.check(),
        &guard,
    );
    assert_eq!(r.state, State::Unknown);
    assert!(r.cleanup_uncertain);
    assert_eq!(client.opens.load(Ordering::SeqCst), 1);
    assert!(!client.events().iter().any(|v| v == "commit"));
}

#[test]
fn final_guard_cancellation_rolls_back_without_commit_dispatch() {
    let original = original();
    let client = Client::new(original.clone(), Fault::None);
    let cancel = client.state.lock().unwrap().cancel.clone();
    let calls = AtomicUsize::new(0);
    let guard = || {
        if calls.fetch_add(1, Ordering::SeqCst) == 3 {
            cancel.store(true, Ordering::SeqCst);
        }
        Ok(())
    };
    let report = stage(
        &client,
        &original,
        &edit(&original),
        &profile(),
        &WritePolicy::new(2, 4096).unwrap(),
        client.check(),
        &guard,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert_eq!(report.state, State::RolledBack);
    assert!(
        report
            .error
            .unwrap()
            .to_string()
            .contains("recorded-cancel")
    );
    assert_eq!(client.opens.load(Ordering::SeqCst), 1);
    assert!(!client.events().iter().any(|event| event == "commit"));
    original
        .require_equal(&client.state.lock().unwrap().db)
        .unwrap();
}

#[test]
fn parameter_value_budget_rejects_below_and_accepts_exact_or_above() {
    // Longest population request: BODY, two I64 parameters and two payload bytes.
    // Use explicit scalar widths independently of the implementation's sum.
    let exact = BODY.len() * 2 + 8 + 2 + 8;
    for budget in [exact - 1, exact, exact + 1] {
        let original = original();
        let client = Client::new(original.clone(), Fault::None);
        let report = stage(
            &client,
            &original,
            &edit(&original),
            &profile(),
            &WritePolicy::new(2, budget).unwrap(),
            client.check(),
            &|| Ok(()),
        );
        if budget < exact {
            assert_eq!(report.state, State::NotPublished);
            assert_eq!(client.opens.load(Ordering::SeqCst), 0);
            original
                .require_equal(&client.state.lock().unwrap().db)
                .unwrap();
        } else {
            assert_eq!(report.state, State::Committed, "{:?}", report.error);
            assert_eq!(client.opens.load(Ordering::SeqCst), 1);
        }
    }
}
