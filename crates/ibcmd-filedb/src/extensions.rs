//! Measured 8.3.27.2214 file extension registry (read-only).
use crate::descr::{FieldKind, Table};
use crate::error::{Error, Result, corrupt};
use crate::{blob, object::Object, pager::Pager, record::decode_var_string};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionRow {
    pub physical_id: [u8; 16],
    pub name: String,
    pub zipped_info: Vec<u8>,
}

const TABLE: &str = "_EXTENSIONSINFO";
const MAX_ROWS: usize = 1024;
const MAX_BLOB: u64 = 8 * 1024 * 1024;
const LAYOUT: [(&str, FieldKind, u32, bool); 10] = [
    ("_IDRREF", FieldKind::Binary, 16, false),
    ("_EXTENSIONORDER", FieldKind::Numeric, 9, false),
    ("_EXTNAME", FieldKind::VarString, 255, true),
    ("_UPDATETIME", FieldKind::DateTime, 0, false),
    ("_EXTENSIONUSEPURPOSE", FieldKind::Numeric, 2, false),
    ("_EXTENSIONSCOPE", FieldKind::Numeric, 2, false),
    ("_EXTENSIONZIPPEDINFO", FieldKind::Image, 0, false),
    ("_MASTERNODE", FieldKind::Text, 0, true),
    ("_USEDINDISTRIBUTEDINFOBASE", FieldKind::Bool, 0, false),
    ("_VERSION", FieldKind::RowVersion, 0, false),
];

pub(crate) fn check_layout(table: &Table) -> Result<()> {
    let supported = table.name == TABLE
        && table.fields.len() == LAYOUT.len()
        && table
            .fields
            .iter()
            .zip(LAYOUT)
            .all(|(f, (name, kind, length, ci))| {
                f.name == name
                    && f.kind == kind
                    && f.length == length
                    && f.case_insensitive == ci
                    && !f.nullable
                    && f.precision == 0
            })
        && table.index_names == ["_IDRREFIDX"];
    if !supported {
        return Err(Error::Refused(
            "Неподдержанный профиль файлового реестра расширений 8.3.27".into(),
        ));
    }
    if [table.records_page, table.blob_page, table.index_page].contains(&0) {
        return Err(corrupt(
            "Реестр расширений: нет объекта записей, blob или индекса",
        ));
    }
    Ok(())
}

pub(crate) fn read(pager: &mut Pager<'_>, table: &Table) -> Result<Vec<ExtensionRow>> {
    check_layout(table)?;
    let size = table.record_size();
    let records = Object::open(pager, table.records_page)?;
    let blobs = Object::open(pager, table.blob_page)?;
    if records.len > ((MAX_ROWS + 1) * size) as u64 || blobs.len > MAX_BLOB {
        return Err(Error::Refused(
            "Реестр расширений превышает поддержанный размер".into(),
        ));
    }
    let records = records.read_all(pager)?;
    let blobs = blobs.read_all(pager)?;
    if records.is_empty() || records.len() % size != 0 {
        return Err(corrupt("Реестр расширений: неверная длина записей"));
    }
    let mut out = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for rec in records.chunks_exact(size).skip(1) {
        match rec[0] {
            1 => continue,
            0 => (),
            _ => return Err(corrupt("Реестр расширений: неверный флаг записи")),
        }
        // Measured file records place RV first, independently of its position
        // in the descriptor. Every other field follows descriptor order.
        let mut offset = 1;
        let mut values = std::collections::BTreeMap::new();
        for f in table
            .fields
            .iter()
            .filter(|f| f.kind == FieldKind::RowVersion)
            .chain(
                table
                    .fields
                    .iter()
                    .filter(|f| f.kind != FieldKind::RowVersion),
            )
        {
            values.insert(f.name.as_str(), &rec[offset..offset + f.size()]);
            offset += f.size();
        }
        let physical_id: [u8; 16] = values["_IDRREF"].try_into().unwrap();
        if physical_id == [0; 16] || !ids.insert(physical_id) {
            return Err(corrupt(
                "Реестр расширений: пустой или повторный идентификатор",
            ));
        }
        let name = decode_var_string(values["_EXTNAME"], 255)?;
        if name.is_empty() || name.contains('\0') {
            return Err(corrupt("Реестр расширений: пустое/неверное имя"));
        }
        for (field, digits) in [
            ("_EXTENSIONORDER", 9),
            ("_EXTENSIONUSEPURPOSE", 2),
            ("_EXTENSIONSCOPE", 2),
        ] {
            let n = crate::record::unpack_decimal(values[field], digits)?;
            if (field == "_EXTENSIONUSEPURPOSE" && n > 2) || (field == "_EXTENSIONSCOPE" && n > 1) {
                return Err(Error::Refused(format!("Неподдержанное значение {field}")));
            }
        }
        if values["_USEDINDISTRIBUTEDINFOBASE"] != [0] {
            return Err(Error::Refused(
                "Распределённый профиль расширения не поддерживается".into(),
            ));
        }
        let master = values["_MASTERNODE"];
        let master_block = u32::from_le_bytes(master[..4].try_into().unwrap());
        let master_len = u32::from_le_bytes(master[4..].try_into().unwrap());
        if master_block == 0 || master_len != 68 {
            return Err(Error::Refused(
                "Неподдержанный профиль главного узла расширения".into(),
            ));
        }
        let master = blob::read_chain(&blobs, master_block, Some(master_len as u64))?;
        let expected: Vec<u8> = format!("0:{}", "0".repeat(32))
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        if master != expected {
            return Err(Error::Refused(
                "Распределённый профиль расширения не поддерживается".into(),
            ));
        }
        let reference = values["_EXTENSIONZIPPEDINFO"];
        let block = u32::from_le_bytes(reference[..4].try_into().unwrap());
        let len = u32::from_le_bytes(reference[4..].try_into().unwrap());
        if block == 0 || len == 0 || len > 4000 {
            return Err(corrupt("Реестр расширений: неверный размер envelope"));
        }
        let zipped_info = blob::read_chain(&blobs, block, Some(len as u64))?;
        out.push(ExtensionRow {
            physical_id,
            name,
            zipped_info,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Field, FieldKind};
    fn table() -> Table {
        let fields = [
            ("_IDRREF", FieldKind::Binary, 16, false),
            ("_EXTENSIONORDER", FieldKind::Numeric, 9, false),
            ("_EXTNAME", FieldKind::VarString, 255, true),
            ("_UPDATETIME", FieldKind::DateTime, 0, false),
            ("_EXTENSIONUSEPURPOSE", FieldKind::Numeric, 2, false),
            ("_EXTENSIONSCOPE", FieldKind::Numeric, 2, false),
            ("_EXTENSIONZIPPEDINFO", FieldKind::Image, 0, false),
            ("_MASTERNODE", FieldKind::Text, 0, true),
            ("_USEDINDISTRIBUTEDINFOBASE", FieldKind::Bool, 0, false),
            ("_VERSION", FieldKind::RowVersion, 0, false),
        ]
        .into_iter()
        .map(|(name, kind, length, case_insensitive)| Field {
            name: name.into(),
            kind,
            nullable: false,
            length,
            precision: 0,
            case_insensitive,
        })
        .collect();
        Table {
            name: "_EXTENSIONSINFO".into(),
            fields,
            index_names: vec!["_IDRREFIDX".into()],
            records_page: 265,
            blob_page: 266,
            index_page: 267,
        }
    }
    #[test]
    fn measured_registry_layout_is_accepted() {
        check_layout(&table()).unwrap();
    }
    #[test]
    fn unknown_registry_profile_is_refused() {
        let mut t = table();
        t.fields[2].length = 254;
        assert!(check_layout(&t).is_err());
    }
}
