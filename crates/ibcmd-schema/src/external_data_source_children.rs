//! ExternalDataSource child descriptor facts (`Cube`, `Table`, `DimensionTable`).
//!
//! The native CF layout below was measured on two independent corpora:
//! the target matrix configuration and the stepwise
//! `TEST123/_ВнешнийИсточникДанных` configurations. The XML order is the
//! platform's Designer order. Keep this table declarative: object identities
//! and property values come from the stored row.

pub const CUBE_KIND: &str = "Cube";
pub const CUBE_COLLECTION: &str = "Cubes";
pub const TABLE_KIND: &str = "Table";
pub const TABLE_COLLECTION: &str = "Tables";
pub const DIMENSION_TABLE_KIND: &str = "DimensionTable";
pub const DIMENSION_TABLE_COLLECTION: &str = "DimensionTables";

pub const CUBE_BODY_ARITY: usize = 27;
pub const TABLE_BODY_ARITY: usize = 52;
pub const DIMENSION_TABLE_BODY_ARITY: usize = 33;

/// The native body record discriminator (`{<code>,...}`).
pub const CUBE_RECORD_CODE: &str = "0";
pub const TABLE_RECORD_CODE: &str = "11";
pub const DIMENSION_TABLE_RECORD_CODE: &str = "0";

/// One generated-type category of a child descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedCategory {
    pub category: &'static str,
    pub name_stem: &'static str,
    /// Index of the TypeId in the owner payload; the ValueId follows it.
    pub body_slot: usize,
    /// Position in the XML `InternalInfo`.
    pub xml_order: usize,
}

/// Cube categories: native body order is Manager, RecordSet, Record,
/// RecordKey, List, DimensionTables, with RecordManager in the tail; the
/// XML order is Manager, List, RecordSet, Record, RecordKey, RecordManager,
/// DimensionTables.
pub const CUBE_GENERATED_CATEGORIES: &[GeneratedCategory] = &[
    GeneratedCategory {
        category: "Manager",
        name_stem: "ExternalDataSourceCubeManager",
        body_slot: 2,
        xml_order: 0,
    },
    GeneratedCategory {
        category: "List",
        name_stem: "ExternalDataSourceCubeList",
        body_slot: 10,
        xml_order: 1,
    },
    GeneratedCategory {
        category: "RecordSet",
        name_stem: "ExternalDataSourceCubeRecordSet",
        body_slot: 4,
        xml_order: 2,
    },
    GeneratedCategory {
        category: "Record",
        name_stem: "ExternalDataSourceCubeRecord",
        body_slot: 6,
        xml_order: 3,
    },
    GeneratedCategory {
        category: "RecordKey",
        name_stem: "ExternalDataSourceCubeRecordKey",
        body_slot: 8,
        xml_order: 4,
    },
    GeneratedCategory {
        category: "RecordManager",
        name_stem: "ExternalDataSourceCubeRecordManager",
        body_slot: 25,
        xml_order: 5,
    },
    GeneratedCategory {
        category: "DimensionTables",
        name_stem: "ExternalDataSourceCubeDimensionsTablesManager",
        body_slot: 12,
        xml_order: 6,
    },
];

/// Table categories: native body order is Manager, RecordSet, Record,
/// RecordKey, Object, Ref, List, with RecordManager in the tail; the XML
/// order is Manager, Object, Ref, List, Record, RecordSet, RecordKey,
/// RecordManager.
pub const TABLE_GENERATED_CATEGORIES: &[GeneratedCategory] = &[
    GeneratedCategory {
        category: "Manager",
        name_stem: "ExternalDataSourceTableManager",
        body_slot: 2,
        xml_order: 0,
    },
    GeneratedCategory {
        category: "Object",
        name_stem: "ExternalDataSourceTableObject",
        body_slot: 10,
        xml_order: 1,
    },
    GeneratedCategory {
        category: "Ref",
        name_stem: "ExternalDataSourceTableRef",
        body_slot: 12,
        xml_order: 2,
    },
    GeneratedCategory {
        category: "List",
        name_stem: "ExternalDataSourceTableList",
        body_slot: 14,
        xml_order: 3,
    },
    GeneratedCategory {
        category: "Record",
        name_stem: "ExternalDataSourceTableRecord",
        body_slot: 6,
        xml_order: 4,
    },
    GeneratedCategory {
        category: "RecordSet",
        name_stem: "ExternalDataSourceTableRecordSet",
        body_slot: 4,
        xml_order: 5,
    },
    GeneratedCategory {
        category: "RecordKey",
        name_stem: "ExternalDataSourceTableRecordKey",
        body_slot: 8,
        xml_order: 6,
    },
    GeneratedCategory {
        category: "RecordManager",
        name_stem: "ExternalDataSourceTableRecordManager",
        body_slot: 36,
        xml_order: 7,
    },
];

pub const DIMENSION_TABLE_GENERATED_CATEGORIES: &[GeneratedCategory] = &[
    GeneratedCategory {
        category: "Manager",
        name_stem: "ExternalDataSourceCubeDimensionTableManager",
        body_slot: 2,
        xml_order: 0,
    },
    GeneratedCategory {
        category: "Object",
        name_stem: "ExternalDataSourceCubeDimensionTableObject",
        body_slot: 4,
        xml_order: 1,
    },
    GeneratedCategory {
        category: "Ref",
        name_stem: "ExternalDataSourceCubeDimensionTableRef",
        body_slot: 6,
        xml_order: 2,
    },
    GeneratedCategory {
        category: "List",
        name_stem: "ExternalDataSourceCubeDimensionTableList",
        body_slot: 8,
        xml_order: 3,
    },
];

/// Root ExternalDataSource collections, in native body order.
pub const ROOT_CUBES_COLLECTION: &str = "2bb208ca-e441-4023-9ab9-32a7807a85d0";
pub const ROOT_FORMULAS_COLLECTION: &str = "a98a108a-49a0-4dcf-bfd0-aa4b89eeb3f5";
pub const ROOT_TABLES_COLLECTION: &str = "e3403acd-1c95-421b-87e4-4dfa29d38b52";

pub const CUBE_FORMS_COLLECTION: &str = "3448e506-5add-4fce-a604-7305466b2d8e";
pub const TABLE_FORMS_COLLECTION: &str = "17816ebc-4068-496e-adc4-8879945a832f";
pub const DIMENSION_TABLE_FORMS_COLLECTION: &str = "2367cec0-5648-49d7-95f0-76d387653517";
pub const TEMPLATES_COLLECTION: &str = "3daea016-69b7-4ed4-9453-127911372fe6";
pub const CUBE_COMMANDS_COLLECTION: &str = "4ee40ec7-3469-439f-adb4-aa26ce2d3ec3";
pub const TABLE_COMMANDS_COLLECTION: &str = "5bb6f09e-5d80-41f6-8070-9faa4d15b69b";
pub const DIMENSION_TABLE_COMMANDS_COLLECTION: &str = "8352a2a3-943d-43a2-95d9-1c592b1d337d";
/// Cube `dimensions` collection: file children (`DimensionTable` XML sidecar).
pub const CUBE_DIMENSION_TABLES_COLLECTION: &str = "631411f3-8c65-4066-a78f-57a72e015c1c";
/// Cube `resources` collection in the Python schema name; the platform writes
/// its items as inline `<Dimension>` elements.
pub const CUBE_INLINE_DIMENSIONS_COLLECTION: &str = "7374aaf6-fe8c-4bba-8574-2075a293f0b6";
/// Cube `dimension_tables` collection in the Python schema name; the platform
/// writes its items as inline `<Resource>` elements.
pub const CUBE_INLINE_RESOURCES_COLLECTION: &str = "cead2290-2080-4077-a57b-73051d618683";
pub const FIELDS_COLLECTION: &str = "c7075688-01c1-41ba-bd1c-1023c8189079";

/// `Cube` -> its generated categories.
#[must_use]
pub const fn categories(kind: &str) -> &'static [GeneratedCategory] {
    match kind.as_bytes() {
        b"Cube" => CUBE_GENERATED_CATEGORIES,
        b"Table" => TABLE_GENERATED_CATEGORIES,
        b"DimensionTable" => DIMENSION_TABLE_GENERATED_CATEGORIES,
        _ => &[],
    }
}

/// `Cube` -> `Cubes`, ...
#[must_use]
pub const fn collection(kind: &str) -> &'static str {
    match kind.as_bytes() {
        b"Cube" => CUBE_COLLECTION,
        b"Table" => TABLE_COLLECTION,
        b"DimensionTable" => DIMENSION_TABLE_COLLECTION,
        _ => "",
    }
}

/// `Cube` -> 27, ...
#[must_use]
pub const fn body_arity(kind: &str) -> usize {
    match kind.as_bytes() {
        b"Cube" => CUBE_BODY_ARITY,
        b"Table" => TABLE_BODY_ARITY,
        b"DimensionTable" => DIMENSION_TABLE_BODY_ARITY,
        _ => 0,
    }
}

/// `Cube` -> `0`, `Table` -> `11`, ...
#[must_use]
pub const fn record_code(kind: &str) -> &'static str {
    match kind.as_bytes() {
        b"Cube" => CUBE_RECORD_CODE,
        b"Table" => TABLE_RECORD_CODE,
        b"DimensionTable" => DIMENSION_TABLE_RECORD_CODE,
        _ => "",
    }
}
