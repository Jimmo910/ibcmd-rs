//! `infobase config import --dbms=PostgreSQL` (#370): the import of an XML
//! tree into an infobase on PostgreSQL.
//!
//! The platform's import there stages the configuration in `configsave` the
//! way it does in a file infobase (8.5.1.1529 into an empty database: every
//! row, then `deleted`, `root`, `version`, `versions`; attributes `0`, the
//! local time in `creation`/`modified`), and `config apply` takes the stage.
//! This import builds the same stage with the file infobase's builder
//! (`crate::file_infobase::plan_stage`) over the database's `config` rows,
//! checks it, and replaces `configsave` with it in one transaction.

use anyhow::{Context, Result, anyhow};
use ibcmd_filedb::{ConfigRow, Timestamp};

use crate::cli::InfobaseConfigImportArgs;
use crate::infobase::{ConnectionConfig, InfobaseConfigImportReport};
use crate::sql::postgres::{PgConnection, decode_bytea};

pub(crate) fn import(
    config: &ConnectionConfig,
    args: &InfobaseConfigImportArgs,
) -> Result<InfobaseConfigImportReport> {
    let target = config
        .postgres()?
        .ok_or_else(|| anyhow!("no PostgreSQL database"))?;
    let (source_root, source_version) = crate::file_infobase::import_source(config, args)?;
    let mut connection = PgConnection::connect(&target)?;
    let stage = crate::file_infobase::plan_stage(
        read_rows(&mut connection, "config")?,
        &source_root,
        source_version,
        args,
    )?;
    let before = write_stage(&mut connection, &stage.rows)
        .with_context(|| format!("не удалось записать configsave базы {}", target.database))?;
    let after = stage.rows.len();
    Ok(stage.report(
        config,
        source_root,
        source_version,
        "postgres-configsave-stage",
        (before, after),
        None,
    ))
}

/// Every part of every row of `table`.
fn read_rows(connection: &mut PgConnection, table: &str) -> Result<Vec<ConfigRow>> {
    let now = Timestamp::now_utc();
    let mut rows = Vec::new();
    connection
        .query(
            &format!(
                "SELECT filename::text, partno, datasize, encode(binarydata, 'hex') FROM {table}"
            ),
            &mut |row| {
                let text = |column: usize| -> Result<String> {
                    let value = row
                        .get(column)
                        .and_then(Option::as_deref)
                        .ok_or_else(|| anyhow!("{table}: NULL in column {column}"))?;
                    Ok(String::from_utf8(value.to_vec())?)
                };
                rows.push(ConfigRow {
                    file_name: text(0)?,
                    part_no: text(1)?.parse().context("part number")?,
                    creation: now,
                    modified: now,
                    attributes: 0,
                    data_size: text(2)?.parse().context("data size")?,
                    binary: match row.get(3).and_then(Option::as_deref) {
                        Some(value) => decode_bytea(value)?,
                        None => Vec::new(),
                    },
                });
                Ok(())
            },
        )
        .with_context(|| format!("failed to read the {table} table"))?;
    Ok(rows)
}

/// Replaces `configsave` with `rows` in one transaction; returns the rows it
/// held before.
fn write_stage(connection: &mut PgConnection, rows: &[ConfigRow]) -> Result<usize> {
    connection.query("BEGIN", &mut |_| Ok(()))?;
    let written = (|| -> Result<usize> {
        let mut before = 0;
        connection.query("SELECT count(*) FROM configsave", &mut |row| {
            before = row
                .first()
                .and_then(Option::as_deref)
                .and_then(|value| std::str::from_utf8(value).ok()?.parse().ok())
                .unwrap_or(0);
            Ok(())
        })?;
        connection.query("DELETE FROM configsave", &mut |_| Ok(()))?;
        for row in rows {
            connection.query(&insert_statement(row), &mut |_| Ok(()))?;
        }
        Ok(before)
    })();
    match written {
        Ok(before) => {
            connection.query("COMMIT", &mut |_| Ok(()))?;
            Ok(before)
        }
        Err(error) => {
            let _ = connection.query("ROLLBACK", &mut |_| Ok(()));
            Err(error)
        }
    }
}

fn insert_statement(row: &ConfigRow) -> String {
    let mut hex = String::with_capacity(row.binary.len() * 2);
    for byte in &row.binary {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!(
        "INSERT INTO configsave (filename, creation, modified, attributes, datasize, binarydata, partno) \
         VALUES (CAST('{}' AS mvarchar), LOCALTIMESTAMP(0), LOCALTIMESTAMP(0), {}, {}, decode('{hex}', 'hex'), {})",
        row.file_name.replace('\'', "''"),
        row.attributes,
        row.data_size,
        row.part_no
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_inserted_with_its_name_quoted_and_bytes_in_hex() {
        let row = ConfigRow {
            file_name: "a'b".to_string(),
            part_no: 1,
            creation: Timestamp::now_utc(),
            modified: Timestamp::now_utc(),
            attributes: 0,
            data_size: 20,
            binary: vec![0, 0xab],
        };
        assert_eq!(
            insert_statement(&row),
            "INSERT INTO configsave (filename, creation, modified, attributes, datasize, binarydata, partno) \
             VALUES (CAST('a''b' AS mvarchar), LOCALTIMESTAMP(0), LOCALTIMESTAMP(0), 0, 20, decode('00ab', 'hex'), 1)"
        );
    }
}
