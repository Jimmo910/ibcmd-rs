//! The CONFIG-family tables: CONFIG, CONFIGSAVE, PARAMS, FILES, CONFIGCAS,
//! CONFIGCASSAVE. All share one layout:
//!
//! `FILENAME NVC(128) CI, CREATION DT, MODIFIED DT, ATTRIBUTES N(5),
//! DATASIZE N(20), BINARYDATA I, PARTNO N(10)` with the index
//! `BYNAMENO_<table> (FILENAME, PARTNO)`.
//!
//! The record object starts with record 0: a free flag 1 and the number of the
//! first free record. A free record holds flag 1 and the next free record
//! number (0 ends the chain).

use std::collections::BTreeMap;

use crate::blob;
use crate::descr::{FieldKind, Table};
use crate::error::{Error, Result, corrupt};
use crate::index;
use crate::object::Object;
use crate::pager::Pager;
use crate::record::{
    Timestamp, decode_var_string, encode_var_string, pack_decimal, unpack_decimal,
};

/// One stored part of a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRow {
    pub file_name: String,
    pub part_no: u32,
    pub creation: Timestamp,
    pub modified: Timestamp,
    pub attributes: u32,
    /// The size of the whole row (every part carries the same value).
    pub data_size: u64,
    pub binary: Vec<u8>,
}

const LAYOUT: [(&str, FieldKind, u32); 7] = [
    ("FILENAME", FieldKind::VarString, 128),
    ("CREATION", FieldKind::DateTime, 0),
    ("MODIFIED", FieldKind::DateTime, 0),
    ("ATTRIBUTES", FieldKind::Numeric, 5),
    ("DATASIZE", FieldKind::Numeric, 20),
    ("BINARYDATA", FieldKind::Image, 0),
    ("PARTNO", FieldKind::Numeric, 10),
];

pub(crate) fn check_layout(table: &Table) -> Result<()> {
    let same = table.fields.len() == LAYOUT.len()
        && table
            .fields
            .iter()
            .zip(LAYOUT)
            .all(|(f, (name, kind, len))| {
                f.name == name && f.kind == kind && f.length == len && !f.nullable
            });
    if !same {
        return Err(Error::Refused(format!(
            "Таблица {} устроена не как таблицы конфигурации (FILENAME … PARTNO)",
            table.name
        )));
    }
    if table.records_page == 0 || table.blob_page == 0 || table.index_page == 0 {
        return Err(corrupt(format!(
            "Таблица {}: нет объекта записей, blob или индекса",
            table.name
        )));
    }
    Ok(())
}

// Offsets inside a 301-byte record.
const NAME: usize = 1;
const CREATION: usize = NAME + 258;
const MODIFIED: usize = CREATION + 7;
const ATTRIBUTES: usize = MODIFIED + 7;
const DATASIZE: usize = ATTRIBUTES + 3;
const BINARY: usize = DATASIZE + 11;
const PARTNO: usize = BINARY + 8;
pub(crate) const RECORD: usize = PARTNO + 6;

/// A record holding the defaults of every field: an empty name of spaces,
/// zero dates, packed zeros. Record 0 and free records are this record with
/// the free flag and the next free number written over its start, as in
/// native files.
fn blank_record() -> Vec<u8> {
    let mut r = vec![0u8; RECORD];
    encode_var_string("", 128, &mut r[NAME..CREATION]).expect("empty name fits");
    r[ATTRIBUTES..DATASIZE].copy_from_slice(&pack_decimal(0, 5));
    r[DATASIZE..BINARY].copy_from_slice(&pack_decimal(0, 20));
    r[PARTNO..RECORD].copy_from_slice(&pack_decimal(0, 10));
    r
}

pub(crate) struct RawRecord {
    pub(crate) number: u32,
    pub(crate) row: ConfigRow,
}

pub(crate) fn read_records(
    pager: &mut Pager<'_>,
    table: &Table,
    with_binary: bool,
) -> Result<Vec<RawRecord>> {
    read_records_bounded(pager, table, with_binary, u64::MAX)
}

pub(crate) fn read_records_bounded(
    pager: &mut Pager<'_>,
    table: &Table,
    with_binary: bool,
    max_retained_bytes: u64,
) -> Result<Vec<RawRecord>> {
    check_layout(table)?;
    debug_assert_eq!(table.record_size(), RECORD);
    let records = Object::open(pager, table.records_page)?.read_all(pager)?;
    if records.len() % RECORD != 0 {
        return Err(corrupt(format!(
            "Таблица {}: длина объекта записей {} не кратна {RECORD}",
            table.name,
            records.len()
        )));
    }
    let blob_object = if with_binary {
        Object::open(pager, table.blob_page)?.read_all(pager)?
    } else {
        Vec::new()
    };
    let mut out = Vec::new();
    let mut remaining_bytes = max_retained_bytes;
    for (number, rec) in records.as_chunks::<RECORD>().0.iter().enumerate().skip(1) {
        match rec[0] {
            0 => {}
            1 => continue,
            flag => {
                return Err(corrupt(format!(
                    "Таблица {}: неизвестный флаг записи {number}: {flag:#x}",
                    table.name
                )));
            }
        }
        let file_name = decode_var_string(&rec[NAME..CREATION], 128)?;
        let block = u32::from_le_bytes(rec[BINARY..BINARY + 4].try_into().unwrap());
        let len = u32::from_le_bytes(rec[BINARY + 4..BINARY + 8].try_into().unwrap());
        let binary = if with_binary && block != 0 {
            remaining_bytes = remaining_bytes.checked_sub(u64::from(len)).ok_or_else(|| {
                Error::Refused(format!(
                    "{}: retained binary exceeds its read budget",
                    table.name
                ))
            })?;
            blob::read_chain(&blob_object, block, Some(len as u64))?
        } else if with_binary && len != 0 {
            return Err(corrupt(format!(
                "Строка {file_name}: длина {len} без первого блока"
            )));
        } else {
            Vec::new()
        };
        let part_no = u32::try_from(unpack_decimal(&rec[PARTNO..RECORD], 10)?)
            .map_err(|_| corrupt(format!("Строка {file_name}: PARTNO вне диапазона u32")))?;
        out.push(RawRecord {
            number: number as u32,
            row: ConfigRow {
                file_name,
                part_no,
                creation: Timestamp(rec[CREATION..MODIFIED].try_into().unwrap()),
                modified: Timestamp(rec[MODIFIED..ATTRIBUTES].try_into().unwrap()),
                attributes: unpack_decimal(&rec[ATTRIBUTES..DATASIZE], 5)? as u32,
                data_size: unpack_decimal(&rec[DATASIZE..BINARY], 20)?,
                binary,
            },
        });
    }
    Ok(out)
}

/// Replaces every row of a CONFIG-family table. Everything is computed before
/// the first write, so a refused key or name changes nothing.
pub(crate) fn replace(
    pager: &mut Pager<'_>,
    table: &Table,
    rows: &[ConfigRow],
    collation: index::Collation,
) -> Result<std::collections::BTreeSet<u32>> {
    check_layout(table)?;
    let ps = pager.page_size();
    let mut records_object = Object::open(pager, table.records_page)?;
    let mut blob_object = Object::open(pager, table.blob_page)?;
    let mut index_object = Object::open(pager, table.index_page)?;
    let old_index = index_object.read_all(pager)?;

    let mut seen = BTreeMap::new();
    let mut keys = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        if seen
            .insert((row.file_name.to_lowercase(), row.part_no), i)
            .is_some()
        {
            return Err(Error::Refused(format!(
                "Строка {} часть {} повторяется (имена сравниваются без учёта регистра)",
                row.file_name, row.part_no
            )));
        }
        keys.push(index::config_key(collation, &row.file_name, row.part_no)?);
        if row.attributes > 99_999 {
            return Err(Error::Refused(format!(
                "Строка {}: ATTRIBUTES вне N(5)",
                row.file_name
            )));
        }
    }

    let values: Vec<&[u8]> = rows.iter().map(|r| r.binary.as_slice()).collect();
    let (blob_content, refs) = blob::build(&values, (blob_object.len as usize) / blob::BLOCK)?;

    let old_records = (records_object.len as usize) / RECORD;
    let total = (rows.len() + 1).max(old_records);
    let blank = blank_record();
    let mut records = blank.repeat(total);
    for (i, row) in rows.iter().enumerate() {
        let r = &mut records[(i + 1) * RECORD..(i + 2) * RECORD];
        encode_var_string(&row.file_name, 128, &mut r[NAME..CREATION])?;
        r[CREATION..MODIFIED].copy_from_slice(&row.creation.0);
        r[MODIFIED..ATTRIBUTES].copy_from_slice(&row.modified.0);
        r[ATTRIBUTES..DATASIZE].copy_from_slice(&pack_decimal(row.attributes as u64, 5));
        r[DATASIZE..BINARY].copy_from_slice(&pack_decimal(row.data_size, 20));
        r[BINARY..BINARY + 4].copy_from_slice(&refs[i].0.to_le_bytes());
        r[BINARY + 4..BINARY + 8].copy_from_slice(&refs[i].1.to_le_bytes());
        r[PARTNO..RECORD].copy_from_slice(&pack_decimal(row.part_no as u64, 10));
    }
    // Record 0 and the free chain over the unused tail.
    let first_free = if rows.len() + 1 < total {
        rows.len() as u32 + 1
    } else {
        0
    };
    records[0] = 1;
    records[1..5].copy_from_slice(&first_free.to_le_bytes());
    for n in rows.len() + 1..total {
        let next = if n + 1 < total { n as u32 + 1 } else { 0 };
        records[n * RECORD] = 1;
        records[n * RECORD + 1..n * RECORD + 5].copy_from_slice(&next.to_le_bytes());
    }

    let entries: Vec<(u32, Vec<u8>)> = keys
        .into_iter()
        .enumerate()
        .map(|(i, key)| (i as u32 + 1, key))
        .collect();
    let index_content = index::rebuild(&old_index, ps, entries)?;

    records_object.rewrite(pager, &records)?;
    blob_object.rewrite(pager, &blob_content)?;
    index_object.rewrite(pager, &index_content)?;
    pager.flush_header()?;
    Ok(records_object
        .owned_pages()
        .chain(blob_object.owned_pages())
        .chain(index_object.owned_pages())
        .collect())
}

/// The pages of a table's three objects.
pub(crate) fn table_pages(pager: &mut Pager<'_>, table: &Table) -> Result<Vec<u32>> {
    let mut pages = Vec::new();
    for header in [table.records_page, table.blob_page, table.index_page] {
        if header != 0 {
            pages.extend(Object::open(pager, header)?.owned_pages());
        }
    }
    Ok(pages)
}

/// Rebuilds only the index of a CONFIG-family table from its live records.
pub(crate) fn reindex(
    pager: &mut Pager<'_>,
    table: &Table,
    collation: index::Collation,
) -> Result<()> {
    check_layout(table)?;
    let records = read_records(pager, table, false)?;
    let entries = records
        .iter()
        .map(|r| {
            Ok((
                r.number,
                index::config_key(collation, &r.row.file_name, r.row.part_no)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut index_object = Object::open(pager, table.index_page)?;
    let old = index_object.read_all(pager)?;
    let content = index::rebuild(&old, pager.page_size(), entries)?;
    index_object.rewrite(pager, &content)?;
    pager.flush_header()
}
