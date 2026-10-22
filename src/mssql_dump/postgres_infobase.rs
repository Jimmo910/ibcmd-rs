//! The export of an infobase on PostgreSQL (`--dbms=PostgreSQL`, #369): the
//! rows of its `config` table, with a completed import stage of
//! `configsave` folded in as for a file infobase, become the offline rows
//! every read of the export answers from.
//!
//! The tables are the SQL Server ones under lower-case names (8.5.1.1529
//! `infobase create --dbms=PostgreSQL`): `config`, `configsave`, columns
//! `filename` (1C's `mvarchar`), `creation`, `modified`, `attributes`,
//! `datasize`, `binarydata` (`bytea`), `partno`; the stored bytes are the
//! same raw-deflate rows.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};

use super::offline_rows::OfflineRows;
use crate::sql::postgres::{PgConnection, PgTarget, decode_bytea};

/// The configuration the database publishes, as offline rows; with
/// `stored_only` its `config` table alone (`config save --db`).
pub(crate) fn load(target: &PgTarget, stored_only: bool) -> Result<OfflineRows> {
    let mut connection = PgConnection::connect(target)?;
    let config = read_table(&mut connection, "config")?;
    if stored_only {
        return super::file_infobase::publish(config, &BTreeSet::new(), || Ok(BTreeMap::new()));
    }
    let mut saved_names = BTreeSet::new();
    connection.query(
        "SELECT DISTINCT filename::text FROM configsave",
        &mut |row| {
            saved_names.insert(text(&row, 0)?);
            Ok(())
        },
    )?;
    super::file_infobase::publish(config, &saved_names, || {
        read_table(&mut connection, "configsave")
    })
}

/// Every row of `table`, its parts joined.
fn read_table(connection: &mut PgConnection, table: &str) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut rows = Vec::new();
    connection
        .query(
            &format!("SELECT filename::text, partno, encode(binarydata, 'hex') FROM {table}"),
            &mut |row| {
                let name = text(&row, 0)?;
                let part: u32 = text(&row, 1)?
                    .parse()
                    .with_context(|| format!("{table}: part number of {name}"))?;
                let binary = match row.get(2).and_then(Option::as_deref) {
                    Some(value) => decode_bytea(value)?,
                    None => Vec::new(),
                };
                rows.push((name, part, binary));
                Ok(())
            },
        )
        .with_context(|| format!("failed to read the {table} table"))?;
    super::file_infobase::assemble_parts(rows, table)
}

fn text(row: &[Option<Vec<u8>>], column: usize) -> Result<String> {
    match row.get(column) {
        Some(Some(value)) => Ok(String::from_utf8(value.clone())?),
        Some(None) => bail!("NULL in column {column}"),
        None => bail!("no column {column}"),
    }
}
