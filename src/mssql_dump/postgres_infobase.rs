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
    connection.query(
        "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY",
        &mut |_| Ok(()),
    )?;
    // CONFIG, the CONFIGSAVE census and its lazy payload must belong to the
    // same database generation even when an import/apply commits meanwhile.
    match read_published(&mut connection, stored_only) {
        Ok(rows) => {
            connection.query("COMMIT", &mut |_| Ok(()))?;
            Ok(rows)
        }
        Err(error) => {
            let _ = connection.query("ROLLBACK", &mut |_| Ok(()));
            Err(error)
        }
    }
}

fn read_published(connection: &mut PgConnection, stored_only: bool) -> Result<OfflineRows> {
    let config = read_table(connection, "config")?;
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
        read_table(connection, "configsave")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    fn message(stream: &mut TcpStream, tag: u8, body: &[u8]) {
        stream.write_all(&[tag]).unwrap();
        stream
            .write_all(&((body.len() + 4) as u32).to_be_bytes())
            .unwrap();
        stream.write_all(body).unwrap();
    }

    fn data_row(stream: &mut TcpStream, columns: &[&[u8]]) {
        let mut body = (columns.len() as u16).to_be_bytes().to_vec();
        for column in columns {
            body.extend_from_slice(&(column.len() as u32).to_be_bytes());
            body.extend_from_slice(column);
        }
        message(stream, b'D', &body);
    }

    fn capture(stored_only: bool, fail_config: bool) -> (bool, Vec<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let mut startup = vec![0; u32::from_be_bytes(length) as usize - 4];
            stream.read_exact(&mut startup).unwrap();
            message(&mut stream, b'R', &0u32.to_be_bytes());
            message(&mut stream, b'Z', b"I");
            let mut queries = Vec::new();
            loop {
                let mut header = [0; 5];
                stream.read_exact(&mut header).unwrap();
                let mut body =
                    vec![0; u32::from_be_bytes(header[1..].try_into().unwrap()) as usize - 4];
                stream.read_exact(&mut body).unwrap();
                if header[0] == b'X' {
                    break;
                }
                assert_eq!(header[0], b'Q');
                let query = String::from_utf8(body[..body.len() - 1].to_vec()).unwrap();
                if query.starts_with("SELECT filename")
                    && query.ends_with("FROM config")
                    && fail_config
                {
                    message(&mut stream, b'E', b"SERROR\0Msynthetic read failure\0\0");
                } else {
                    if query.starts_with("SELECT DISTINCT") {
                        data_row(&mut stream, &[b"versions"]);
                    } else if query.ends_with("FROM configsave") {
                        // An invalid inventory still exercises the lazy payload read;
                        // publication safely falls back to the stored configuration.
                        data_row(&mut stream, &[b"versions", b"0", b"00"]);
                    }
                    message(&mut stream, b'C', b"OK\0");
                }
                message(&mut stream, b'Z', b"I");
                queries.push(query);
            }
            queries
        });
        let target = PgTarget::new(&address.to_string(), "synthetic", None, None).unwrap();
        let ok = load(&target, stored_only).is_ok();
        (ok, server.join().unwrap())
    }

    #[test]
    fn postgres_export_uses_one_read_only_repeatable_read_snapshot() {
        let (ok, queries) = capture(false, false);
        assert!(ok);
        assert_eq!(
            queries.first().map(String::as_str),
            Some("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
        );
        assert!(queries[1].ends_with("FROM config"));
        assert_eq!(queries[2], "SELECT DISTINCT filename::text FROM configsave");
        assert!(queries[3].ends_with("FROM configsave"));
        assert_eq!(queries.last().map(String::as_str), Some("COMMIT"));
        assert_eq!(queries.len(), 5);
    }

    #[test]
    fn postgres_stored_only_still_uses_a_read_only_snapshot() {
        let (ok, queries) = capture(true, false);
        assert!(ok);
        assert_eq!(
            queries.first().map(String::as_str),
            Some("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
        );
        assert_eq!(queries.last().map(String::as_str), Some("COMMIT"));
        assert_eq!(queries.len(), 3);
    }

    #[test]
    fn postgres_read_failure_rolls_back_without_publishing() {
        let (ok, queries) = capture(false, true);
        assert!(!ok);
        assert_eq!(queries.last().map(String::as_str), Some("ROLLBACK"));
    }
}
