//! Single-request read-only SQL acquisition. Not wired to editor dispatch.
//!
//! There is deliberately no accepted built-in profile. The operator must measure
//! the exact SQL version and schema hash first. Recording clients establish the
//! decoder contract only; lock/coherence/cancellation behavior needs real SQL.
use super::readset::{Builder, Check, Input, Key, Limits, Part, Readset, Table};
use crate::{
    mssql_live_continue::DatabaseIdentity,
    sql::{Dbms, SqlClient, SqlParam, SqlRow, SqlSession, SqlValue},
};
use anyhow::{Result, bail, ensure};
use std::sync::Arc;

/// Operator-owned measured profile, never accepted from an import RPC argument.
pub struct Profile {
    pub identity: DatabaseIdentity,
    pub sql_version: String,
    /// SHA-256 of the batch's canonical schema JSON in UTF-16LE (SQL HASHBYTES).
    pub schema: [u8; 32],
}
const MAX_SCHEMA_BYTES: i64 = 32 * 1024;

/// One call to read_rows retains the server transaction until the full tables
/// have streamed. Any error, incomplete seal or cancellation discards all rows.
/// SqlClient must discard its connection on callback failure (the built-in TDS
/// client does); no separate query/execute calls or retries are used here.
pub fn capture(
    client: &dyn SqlClient,
    profile: &Profile,
    limits: Limits,
    check: Check,
) -> Result<Readset> {
    check()?;
    ensure!(
        client.dbms() == Dbms::SqlServer,
        "snapshot requires SQL Server"
    );
    ensure!(
        client.detached_reason().is_none(),
        "snapshot requires a connected SQL client"
    );
    Ok(
        capture_inner(profile, limits, check, false, |query, params, each| {
            client.read_rows(query, params, each)
        })?
        .readset,
    )
}

pub(super) struct HeldReadset {
    pub readset: Readset,
    pub session_id: i64,
}
struct Captured {
    readset: Readset,
    session_id: Option<i64>,
}

pub(super) fn capture_held(
    session: &mut dyn SqlSession,
    profile: &Profile,
    limits: Limits,
    check: Check,
) -> Result<HeldReadset> {
    let captured = capture_inner(profile, limits, check, true, |query, params, each| {
        session.read_rows(query, params, each)
    })?;
    Ok(HeldReadset {
        readset: captured.readset,
        session_id: captured
            .session_id
            .ok_or_else(|| anyhow::anyhow!("held SQL session seal is missing"))?,
    })
}

fn held_query() -> Result<String> {
    // Reuse the exact schema/row producer. Fail closed if a canonical control
    // anchor changes; never silently fork its serializer or rollback behavior.
    let mut sql = include_str!("snapshot.sql").to_owned();
    for (from, to, count) in [
        ("USE __DATABASE__;\n", "", 1),
        (
            "IF @@TRANCOUNT <> 0 THROW 57620, 'snapshot requires an idle session', 1;",
            "IF @@TRANCOUNT <> 1 OR XACT_STATE() <> 1 THROW 57620, 'held snapshot requires one committable transaction', 1;",
            1,
        ),
        (" BEGIN TRANSACTION;\n", "", 1),
        ("WITH (TABLOCK,HOLDLOCK)", "WITH (TABLOCKX,HOLDLOCK)", 2),
        (
            " ROLLBACK TRANSACTION;\n SELECT @P7,@NC,@NS,@BC,@BS;",
            " SELECT @P7,@NC,@NS,@BC,@BS,@@TRANCOUNT,XACT_STATE(),@@SPID;",
            1,
        ),
    ] {
        ensure!(
            sql.matches(from).count() == count,
            "canonical snapshot control anchor changed"
        );
        sql = sql.replace(from, to);
    }
    Ok(sql)
}

fn capture_inner(
    profile: &Profile,
    limits: Limits,
    check: Check,
    held: bool,
    run: impl FnOnce(&str, &[SqlParam<'_>], &mut dyn FnMut(SqlRow) -> Result<()>) -> Result<()>,
) -> Result<Captured> {
    check()?;
    let db = &profile.identity.database;
    ensure!(
        !db.is_empty() && !db.contains('\0') && db.encode_utf16().count() <= 128,
        "invalid snapshot database identifier"
    );
    ensure!(
        !profile.sql_version.is_empty() && profile.sql_version.len() <= 128,
        "snapshot SQL version is not measured"
    );
    // Validate bounds/identity before touching the client.
    let _ = Builder::new(
        profile.identity.clone(),
        profile.schema,
        limits,
        check.clone(),
    )?;
    let count = i64::try_from(limits.max_parts)?;
    ensure!(count < i64::MAX, "snapshot part limit is too large");
    let part = i64::try_from(limits.max_part_bytes)?;
    let payload = i64::try_from(limits.max_payload_bytes)?;
    // Builder reserves these exact UTF-8 identity bytes before row admission.
    // Give SQL only the remaining budget, so it cannot stream payload that
    // would fail cumulative metadata admission at the client boundary.
    let identity_bytes = [
        &profile.identity.server,
        &profile.identity.database,
        &profile.identity.database_guid,
        &profile.identity.family_guid,
        &profile.identity.recovery_fork,
    ]
    .into_iter()
    .try_fold(0u64, |sum, value| sum.checked_add(value.len() as u64))
    .ok_or_else(|| anyhow::anyhow!("SQL snapshot identity size overflow"))?;
    let metadata = i64::try_from(
        limits
            .max_metadata_bytes
            .checked_sub(identity_bytes)
            .ok_or_else(|| anyhow::anyhow!("SQL snapshot identity exceeds metadata budget"))?,
    )?;
    let name = i64::try_from(limits.max_name_units)?;
    ensure!(name <= 128, "snapshot name limit exceeds storage profile");
    let nonce = uuid::Uuid::new_v4().to_string();
    let query = if held {
        held_query()?
    } else {
        include_str!("snapshot.sql")
            .replace("__DATABASE__", &format!("[{}]", db.replace(']', "]]")))
    };
    let params = [
        SqlParam::Binary(&profile.schema),
        SqlParam::I64(count),
        SqlParam::I64(part),
        SqlParam::I64(payload),
        SqlParam::I64(metadata),
        SqlParam::I64(name),
        SqlParam::Text(&nonce),
        SqlParam::I64(MAX_SCHEMA_BYTES),
        SqlParam::Text(&profile.sql_version),
        SqlParam::Text(&profile.identity.server),
        SqlParam::Text(&profile.identity.database),
        SqlParam::Text(&profile.identity.database_guid),
        SqlParam::Text(&profile.identity.family_guid),
        SqlParam::Text(&profile.identity.recovery_fork),
    ];
    let mut decoder = Decoder {
        profile,
        limits,
        check,
        nonce: &nonce,
        builder: None,
        last_set: 0,
        counts: [0; 2],
        bytes: [0; 2],
        sealed: false,
        held,
        session_id: None,
    };
    let mut failed = None;
    let outcome = run(&query, &params, &mut |row| {
        if failed.is_some() {
            bail!("SQL snapshot stream was invalidated");
        }
        if let Err(error) = decoder.accept(row) {
            // Retain the original typed cancellation even if a client wraps or
            // accidentally ignores callback errors. Such a stream cannot seal.
            failed = Some(error);
            bail!("SQL snapshot stream was invalidated");
        }
        Ok(())
    });
    if let Some(error) = failed {
        return Err(error);
    }
    outcome?;
    (decoder.check)()?;
    ensure!(decoder.sealed, "SQL snapshot stream has no completion seal");
    let session_id = decoder.session_id;
    let readset = decoder
        .builder
        .ok_or_else(|| anyhow::anyhow!("SQL snapshot identity is missing"))?
        .finish()?;
    Ok(Captured {
        readset,
        session_id,
    })
}

struct Decoder<'a> {
    profile: &'a Profile,
    limits: Limits,
    check: Check,
    nonce: &'a str,
    builder: Option<Builder>,
    last_set: usize,
    counts: [u64; 2],
    bytes: [u64; 2],
    sealed: bool,
    held: bool,
    session_id: Option<i64>,
}
impl Decoder<'_> {
    fn accept(&mut self, row: SqlRow) -> Result<()> {
        (self.check)()?;
        ensure!(
            !self.sealed && row.result_set >= self.last_set,
            "SQL snapshot frame order is invalid"
        );
        self.last_set = row.result_set;
        match row.result_set {
            0 => {
                ensure!(self.builder.is_none(), "SQL snapshot identity repeated");
                let [server, database, guid, family, fork, version, schema]: [SqlValue; 7] = row
                    .values
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("SQL snapshot metadata shape is invalid"))?;
                let identity = DatabaseIdentity {
                    server: text(server)?,
                    database: text(database)?,
                    database_guid: text(guid)?,
                    family_guid: text(family)?,
                    recovery_fork: text(fork)?,
                };
                ensure!(
                    identity == self.profile.identity,
                    "SQL snapshot target identity changed"
                );
                ensure!(
                    text(version)? == self.profile.sql_version,
                    "SQL snapshot version is not measured"
                );
                ensure!(
                    binary(schema)?.as_slice() == self.profile.schema,
                    "SQL snapshot schema is not measured"
                );
                self.builder = Some(Builder::new(
                    identity,
                    self.profile.schema,
                    self.limits,
                    self.check.clone(),
                )?);
            }
            1 | 2 => {
                let index = row.result_set - 1;
                let [name, part, size, creation, modified, attributes, blob]: [SqlValue; 7] = row
                    .values
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("SQL snapshot part shape is invalid"))?;
                let file_name = text(name)?;
                let part_no = u32::try_from(integer(part)?)?;
                let data_size = u64::try_from(integer(size)?)?;
                let mut header = Vec::new();
                for value in [creation, modified, attributes] {
                    let bytes = binary(value)?;
                    ensure!(
                        !bytes.is_empty() && bytes.len() <= 64,
                        "SQL snapshot header field is unsupported"
                    );
                    header.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
                    header.extend_from_slice(&bytes);
                }
                let blob = binary(blob)?;
                self.counts[index] = self.counts[index]
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("SQL snapshot count overflow"))?;
                self.bytes[index] = self.bytes[index]
                    .checked_add(blob.len() as u64)
                    .ok_or_else(|| anyhow::anyhow!("SQL snapshot byte count overflow"))?;
                self.builder
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("SQL snapshot parts preceded identity"))?
                    .push(
                        if index == 0 {
                            Table::Config
                        } else {
                            Table::ConfigSave
                        },
                        Input {
                            key: Key { file_name, part_no },
                            part: Part {
                                data_size,
                                header,
                                binary: Arc::new(blob),
                            },
                        },
                    )?;
            }
            3 => {
                let mut row = row;
                if self.held {
                    ensure!(row.values.len() == 8, "held SQL seal shape is invalid");
                    let spid = integer(row.values.pop().unwrap())?;
                    let state = integer(row.values.pop().unwrap())?;
                    let count = integer(row.values.pop().unwrap())?;
                    ensure!(
                        count == 1 && state == 1 && spid > 0,
                        "held SQL transaction/session state changed"
                    );
                    self.session_id = Some(spid);
                }
                let [nonce, nc, ns, bc, bs]: [SqlValue; 5] = row
                    .values
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("SQL snapshot seal shape is invalid"))?;
                ensure!(
                    text(nonce)? == self.nonce,
                    "SQL snapshot completion nonce changed"
                );
                let counts = [u64::try_from(integer(nc)?)?, u64::try_from(integer(ns)?)?];
                let bytes = [u64::try_from(integer(bc)?)?, u64::try_from(integer(bs)?)?];
                ensure!(
                    counts == self.counts && bytes == self.bytes,
                    "SQL snapshot completion counts differ"
                );
                ensure!(self.builder.is_some(), "SQL snapshot identity is missing");
                self.sealed = true;
            }
            _ => bail!("SQL snapshot resultset is unsupported"),
        }
        Ok(())
    }
}
fn text(value: SqlValue) -> Result<String> {
    match value {
        SqlValue::Text(value) => Ok(value),
        _ => bail!("SQL snapshot expected text"),
    }
}
fn binary(value: SqlValue) -> Result<Vec<u8>> {
    match value {
        SqlValue::Binary(value) => Ok(value),
        _ => bail!("SQL snapshot expected binary"),
    }
}
fn integer(value: SqlValue) -> Result<i64> {
    match value {
        SqlValue::Int(value) => Ok(value),
        _ => bail!("SQL snapshot expected integer"),
    }
}
