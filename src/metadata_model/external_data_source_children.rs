//! ExternalDataSource child descriptors (`Cube`, `Table`, `DimensionTable`):
//! native row -> Designer XML.
//!
//! The native body layouts and generated-type slots are measured on the target
//! matrix configuration and on the stepwise `_ВнешнийИсточникДанных` corpus.
//! Property values that are not yet decoded are written as the platform's
//! defaults only after the stored row was checked against the measured shape;
//! an unknown shape is refused instead of silently rewritten.

use anyhow::{Result, anyhow, bail};
use ibcmd_schema::external_data_source_children as schema;

use super::attribute::{CHOICE_HISTORY_ON_INPUT, CREATE_ON_INPUT, FILL_CHECKING, QUICK_CHOICE};
use super::brace::{Brace, parse_row};
use super::export::values::{
    Header, Owner, attribute_properties, bool_text, code_text, header, header_elements,
    localized_element, reference_name, type_element, value_element,
};
use super::export::{
    Build, ExportContext, GeneratedTypeName, NameIndex, ObjectNames, atom, el, item, leaf, list,
    string, xml_text,
};
use super::xml::Element;

/// Type id of a `KeyFields` typed reference list.
const KEY_FIELD_REF_TYPE: &str = "157fa490-4ce9-11d4-9415-008048da11f9";
/// Type id of an `InputByString` typed reference list (stored one level deeper).
const INPUT_BY_STRING_REF_TYPE: &str = "60ea359f-3a6e-48bb-8e71-d2a457572918";

/// The owner payload of a child descriptor: `{<code>,<header>,<types...>,...}`.
struct Row<'a> {
    root: &'a [Brace],
    payload: &'a [Brace],
}

impl<'a> Row<'a> {
    fn parse(row: &'a Brace, kind: &str) -> Result<Self> {
        let root = list(row)?;
        if atom(item(root, 0)?)? != "1" {
            bail!("{kind}: not a descriptor row");
        }
        let payload = list(item(root, 1)?)?;
        if payload.len() != schema::body_arity(kind)
            || atom(item(payload, 0)?)? != schema::record_code(kind)
        {
            bail!("{kind}: unexpected body layout");
        }
        Ok(Self { root, payload })
    }

    fn header(&self, kind: &str) -> Result<Header> {
        let wrapper = list(item(self.payload, 1)?)?;
        let head = list(item(wrapper, 1)?)?;
        let _ = kind;
        header(&Brace::List(head.to_vec()))
    }

    fn items(&self, collection: &str) -> Result<&'a [Brace]> {
        let count = usize::try_from(super::export::number(item(self.root, 2)?)?)
            .map_err(|_| anyhow!("collection count does not fit"))?;
        let fields = self
            .root
            .iter()
            .skip(3)
            .take(count)
            .find_map(|node| {
                let fields = node.as_list()?;
                (fields.first()?.as_atom()? == collection).then_some(fields)
            })
            .ok_or_else(|| anyhow!("no {collection} collection"))?;
        let count = usize::try_from(super::export::number(item(fields, 1)?)?)
            .map_err(|_| anyhow!("item count does not fit"))?;
        fields
            .get(2..2 + count)
            .ok_or_else(|| anyhow!("{collection} is shorter than {count}"))
    }

    fn has_collection(&self, collection: &str) -> bool {
        self.items(collection).is_ok()
    }
}

/// The path segments of an EDS child: root, cube and (for a dimension table)
/// the dimension-table name.
struct Path {
    root: String,
    cube: Option<String>,
    table: Option<String>,
    dimension: Option<String>,
}

impl Path {
    fn from_full(kind: &str, full: &str) -> Result<Self> {
        let parts = full.split('.').collect::<Vec<_>>();
        let expected = match kind {
            schema::CUBE_KIND => 4,
            schema::TABLE_KIND => 4,
            schema::DIMENSION_TABLE_KIND => 6,
            other => bail!("unknown EDS child kind {other}"),
        };
        if parts.len() != expected || parts.first().copied() != Some("ExternalDataSource") {
            bail!("{kind}: unexpected full name {full}");
        }
        Ok(match kind {
            schema::CUBE_KIND => Self {
                root: parts[1].to_string(),
                cube: Some(parts[3].to_string()),
                table: None,
                dimension: None,
            },
            schema::TABLE_KIND => Self {
                root: parts[1].to_string(),
                cube: None,
                table: Some(parts[3].to_string()),
                dimension: None,
            },
            schema::DIMENSION_TABLE_KIND => {
                if parts[2] != schema::CUBE_KIND || parts[4] != schema::DIMENSION_TABLE_KIND {
                    bail!("{kind}: unexpected full name {full}");
                }
                Self {
                    root: parts[1].to_string(),
                    cube: Some(parts[3].to_string()),
                    table: None,
                    dimension: Some(parts[5].to_string()),
                }
            }
            _ => unreachable!(),
        })
    }

    fn generated_name(&self, kind: &str, stem: &str) -> Result<String> {
        Ok(match kind {
            schema::CUBE_KIND => format!(
                "{stem}.{}.{}",
                self.root,
                self.cube.as_deref().ok_or_else(|| anyhow!("cube name"))?
            ),
            schema::TABLE_KIND => format!(
                "{stem}.{}.{}",
                self.root,
                self.table.as_deref().ok_or_else(|| anyhow!("table name"))?
            ),
            schema::DIMENSION_TABLE_KIND => format!(
                "{stem}.{}.{}.{}",
                self.root,
                self.cube.as_deref().ok_or_else(|| anyhow!("cube name"))?,
                self.dimension
                    .as_deref()
                    .ok_or_else(|| anyhow!("dimension-table name"))?
            ),
            other => bail!("unknown EDS child kind {other}"),
        })
    }
}

fn full_name(names: &NameIndex, uuid: &str) -> Result<String> {
    names
        .name(uuid)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("no name for EDS child {uuid}"))
}

fn short_name(names: &NameIndex, uuid: &str) -> Result<String> {
    let full = full_name(names, uuid)?;
    full.rsplit('.')
        .next()
        .map(str::to_string)
        .ok_or_else(|| anyhow!("empty name"))
}

fn generated_type(name: String, category: &str, type_id: &str, value_id: &str) -> Element {
    el("xr:GeneratedType")
        .attr("name", name)
        .attr("category", category)
        .child(leaf("xr:TypeId", type_id))
        .child(leaf("xr:ValueId", value_id))
}

fn internal_info(kind: &str, path: &Path, payload: &[Brace]) -> Result<Element> {
    let mut info = el("InternalInfo");
    for category in schema::categories(kind) {
        let type_id = atom(item(payload, category.body_slot)?)?;
        let value_id = atom(item(payload, category.body_slot + 1)?)?;
        info.children.push(generated_type(
            path.generated_name(kind, category.name_stem)?,
            category.category,
            type_id,
            value_id,
        ));
    }
    Ok(info)
}

fn bool_element(name: &str, node: &Brace) -> Result<Element> {
    Ok(leaf(name, bool_text(node)?))
}

/// Cube properties, in Designer order. Every value except NameInDataSource is
/// the measured default of both corpora; the row shape is checked first.
fn cube_properties(row: &Row<'_>, head: &Header) -> Result<Element> {
    let p = row.payload;
    let [name, synonym, comment] = header_elements(head)?;
    check_row_shape(row, "Cube")?;
    Ok(el("Properties")
        .child(name)
        .child(synonym)
        .child(comment)
        .child(leaf("NameInDataSource", xml_text(string(item(p, 14)?)?)))
        .child(el("Characteristics"))
        .child(bool_element("UseStandardCommands", item(p, 16)?)?)
        .child(el("DefaultRecordForm"))
        .child(el("DefaultListForm"))
        .child(el("RecordPresentation"))
        .child(el("ExtendedRecordPresentation"))
        .child(el("ListPresentation"))
        .child(el("ExtendedListPresentation"))
        .child(el("Explanation"))
        .child(leaf("IncludeHelpInContents", "false")))
}

/// Table properties, in Designer order. Values that are not decoded below are
/// the measured default of both corpora; `check_row_shape` refuses others.
fn table_properties(row: &Row<'_>, head: &Header, full: &str) -> Result<Element> {
    let p = row.payload;
    let [name, synonym, comment] = header_elements(head)?;
    check_row_shape(row, "Table")?;
    // Two native layouts are measured: the matrix stores `1` (8.3.27), the
    // ServiceDesk row stores `0` (8.5.1). Both are the `Table` value; the
    // `Expression` code is not measured and is refused.
    let table_type = match atom(item(p, 16)?)? {
        "0" | "1" => "Table",
        other => bail!("Table: unknown TableType code {other}"),
    };
    let table_data_type = match atom(item(p, 20)?)? {
        "0" => "NonobjectData",
        "1" => "ObjectData",
        other => bail!("Table: unknown TableDataType code {other}"),
    };
    let (key_fields, input_by_string) = table_ref_lists(row)?;
    Ok(el("Properties")
        .child(name)
        .child(synonym)
        .child(comment)
        .child(leaf("TableType", table_type))
        .child(leaf("NameInDataSource", xml_text(string(item(p, 17)?)?)))
        .child(el("ExpressionInDataSource"))
        .child(leaf("TableDataType", table_data_type))
        .child(el("KeyFields").children(key_field_elements(full, &key_fields)))
        .child(el("PresentationField"))
        .child(el("ParentField"))
        .child(el("UnfilledParentValue").attr("nil", "true"))
        .child(el("Characteristics"))
        .child(bool_element("UseStandardCommands", item(p, 21)?)?)
        .child(bool_element("QuickChoice", item(p, 22)?)?)
        .child(el("InputByString").children(key_field_elements(full, &input_by_string)))
        .child(leaf("CreateOnInput", "Auto"))
        .child(leaf("SearchStringModeOnInputByString", "Begin"))
        .child(leaf("ChoiceDataGetModeOnInputByString", "Directly"))
        .child(leaf("ChoiceHistoryOnInput", "Auto"))
        .child(el("DefaultObjectForm"))
        .child(el("DefaultRecordForm"))
        .child(el("DefaultListForm"))
        .child(el("DefaultChoiceForm"))
        .child(el("ObjectPresentation"))
        .child(el("ExtendedObjectPresentation"))
        .child(el("RecordPresentation"))
        .child(el("ExtendedRecordPresentation"))
        .child(el("ListPresentation"))
        .child(el("ExtendedListPresentation"))
        .child(el("Explanation"))
        .child(leaf("IncludeHelpInContents", "false"))
        .child(leaf("ReadOnly", "false"))
        .child(leaf("TransactionsIsolationLevel", "Auto"))
        .child(el("DataVersionField"))
        .child(leaf("EditType", "InDialog"))
        .child(el("BasedOn"))
        .child(el("DataLockFields"))
        .child(leaf("DataLockControlMode", "Automatic")))
}

fn key_field_elements(full: &str, fields: &[String]) -> Vec<Element> {
    fields
        .iter()
        .map(|name| leaf("xr:Field", format!("{full}.Field.{name}")))
        .collect()
}

/// `KeyFields` and `InputByString` as field names, in stored order.
fn table_ref_lists(row: &Row<'_>) -> Result<(Vec<String>, Vec<String>)> {
    let names = field_names(row)?;
    let resolve = |ids: Vec<String>| -> Result<Vec<String>> {
        ids.into_iter()
            .map(|id| {
                names
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| anyhow!("Table: typed list names unknown field {id}"))
            })
            .collect()
    };
    let mut key_fields = Vec::new();
    for node in row.payload.iter().skip(2) {
        if let Some(ids) = typed_ref_list(node, KEY_FIELD_REF_TYPE) {
            key_fields = resolve(ids)?;
        }
    }
    let input_by_string =
        find_typed_ref_list(&Brace::List(row.payload.to_vec()), INPUT_BY_STRING_REF_TYPE)
            .map(resolve)
            .transpose()?
            .unwrap_or_default();
    Ok((key_fields, input_by_string))
}

fn field_names(row: &Row<'_>) -> Result<std::collections::BTreeMap<String, String>> {
    let mut names = std::collections::BTreeMap::new();
    for entry in row.items(schema::FIELDS_COLLECTION).unwrap_or_default() {
        let wrapper = list(entry)?;
        let record = list(item(wrapper, 0)?)?;
        let body = item(record, 1)?;
        let typed = list(item(list(body)?, 1)?)?;
        let head = header(item(typed, 1)?)?;
        names.insert(head.uuid, head.name);
    }
    Ok(names)
}

/// `{0,<count>,{"#",<type>,{<kind>,<uuid>}}…}` -> field UUIDs of `type`.
fn typed_ref_list(node: &Brace, type_id: &str) -> Option<Vec<String>> {
    let items = list(node).ok()?;
    if items.first().and_then(Brace::as_atom) != Some("0") {
        return None;
    }
    let mut identities = Vec::new();
    for entry in items.iter().skip(2) {
        let typed = list(entry).ok()?;
        if typed.first().and_then(Brace::as_str) != Some("#")
            || typed.get(1).and_then(Brace::as_atom) != Some(type_id)
        {
            return None;
        }
        let inner = list(typed.get(2)?).ok()?;
        identities.push(inner.get(1).and_then(Brace::as_atom)?.to_string());
    }
    // An empty `{0,0,…}` list of the same shape is not the property.
    (!identities.is_empty()).then_some(identities)
}

fn find_typed_ref_list(node: &Brace, type_id: &str) -> Option<Vec<String>> {
    if let Some(identities) = typed_ref_list(node, type_id) {
        return Some(identities);
    }
    let items = list(node).ok()?;
    for child in items {
        if let Some(identities) = find_typed_ref_list(child, type_id) {
            return Some(identities);
        }
    }
    None
}

/// DimensionTable properties, in Designer order. Every value except
/// NameInDataSource is the measured default of both corpora.
fn dimension_properties(row: &Row<'_>, head: &Header) -> Result<Element> {
    let p = row.payload;
    let [name, synonym, comment] = header_elements(head)?;
    check_row_shape(row, "DimensionTable")?;
    Ok(el("Properties")
        .child(name)
        .child(synonym)
        .child(comment)
        .child(leaf("NameInDataSource", xml_text(string(item(p, 13)?)?)))
        .child(el("PresentationField"))
        .child(el("HierarchyNameInDataSource"))
        .child(leaf(
            "LevelNumber",
            super::export::number(item(p, 16)?)?.to_string(),
        ))
        .child(bool_element("Hierarchical", item(p, 17)?)?)
        .child(el("UnfilledParentValue").attr("nil", "true"))
        .child(leaf("UseStandardCommands", "false"))
        .child(leaf("QuickChoice", "false"))
        .child(el("DefaultObjectForm"))
        .child(el("DefaultListForm"))
        .child(el("DefaultChoiceForm"))
        .child(el("ObjectPresentation"))
        .child(el("ExtendedObjectPresentation"))
        .child(el("ListPresentation"))
        .child(el("ExtendedListPresentation"))
        .child(el("Explanation"))
        .child(leaf("IncludeHelpInContents", "false")))
}

/// Refuse a row whose un-decoded property slots are not the measured defaults.
/// This keeps the exporter from silently publishing a default where the source
/// held something else until the slot is evidenced.
fn check_row_shape(row: &Row<'_>, kind: &str) -> Result<()> {
    let p = row.payload;
    match kind {
        "Cube" => {
            if list(item(p, 24)?)?.len() != 2 {
                bail!("Cube: unverified slot 24 changed");
            }
        }
        "Table" => {
            // Slots 18, 19, 23..35, 38..51 are not decoded yet. Their measured
            // values are compared structurally where they are lists and by the
            // exact scalar values where they are atoms.
            for index in [
                18usize, 19, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 38, 39, 40, 41,
                42, 43, 44, 45, 46, 47, 48, 49, 50, 51,
            ] {
                if item(p, index).is_err() {
                    bail!("Table: short row at slot {index}");
                }
            }
            if list(item(p, 18)?)?.first().and_then(Brace::as_atom) != Some("0") {
                bail!("Table: unverified KeyFields slot changed");
            }
            if !is_list_of(item(p, 32)?, 2) || !is_list_of(item(p, 33)?, 2) {
                bail!("Table: unverified presentation slots changed");
            }
            if !is_list_of(item(p, 44)?, 4) {
                bail!("Table: unverified edit slots changed");
            }
            if !is_list_of(item(p, 49)?, 2) || !is_list_of(item(p, 50)?, 2) {
                bail!("Table: unverified lock slots changed");
            }
        }
        "DimensionTable" => {
            // Slots 14 (PresentationField reference to MEMBER_UNIQUE_NAME),
            // 28 (PARENT_UNIQUE_NAME identity) and the built-in field pair
            // 31/32 are structural. The remaining slots are measured defaults.
            for index in [
                10usize, 11, 12, 15, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 29, 30,
            ] {
                if item(p, index).is_err() {
                    bail!("DimensionTable: short row at slot {index}");
                }
            }
            if list(item(p, 14)?)?.len() != 3
                || list(item(p, 25)?)?.len() != 2
                || list(item(p, 31)?)?.len() != 2
                || list(item(p, 32)?)?.len() != 2
            {
                bail!("DimensionTable: unverified structural slots changed");
            }
        }
        other => bail!("unknown EDS child kind {other}"),
    }
    Ok(())
}

fn is_list_of(node: &Brace, len: usize) -> bool {
    node.as_list().is_some_and(|fields| fields.len() == len)
}

/// `{27,{2,<header>,<pattern>},...}` -> `<Properties>` for an inline Dimension.
fn inline_dimension_properties(
    record: &[Brace],
    owner: Owner<'_>,
    context: &ExportContext,
) -> Result<Element> {
    let body = item(record, 1)?;
    attribute_properties(body, true, Vec::new(), owner, context)
}

/// Cube `resources` collection item: `{0,{27,...},"<NameInDataSource>"}`,
/// written as the platform's inline `<Resource>`.
fn inline_resource_properties(
    record: &[Brace],
    owner: Owner<'_>,
    context: &ExportContext,
) -> Result<Element> {
    let body = item(record, 1)?;
    let slots = list(body)?;
    if atom(item(slots, 0)?)? != "27" || slots.len() != 23 {
        bail!("Resource: not an attribute body");
    }
    let typed = list(item(slots, 1)?)?;
    let head = header(item(typed, 1)?)?;
    let [name, synonym, comment] = header_elements(&head)?;
    let names = &context.names;
    Ok(el("Properties")
        .child(name)
        .child(synonym)
        .child(comment)
        .child(type_element("Type", item(typed, 2)?, names)?)
        .child(bool_element("PasswordMode", item(slots, 2)?)?)
        .child(localized_element("Format", item(slots, 3)?)?)
        .child(localized_element("EditFormat", item(slots, 18)?)?)
        .child(localized_element("ToolTip", item(slots, 4)?)?)
        .child(bool_element("MarkNegatives", item(slots, 5)?)?)
        .child(leaf("Mask", xml_text(string(item(slots, 6)?)?)))
        .child(bool_element("MultiLine", item(slots, 7)?)?)
        .child(super::export::values::choice_parameter_links_element(
            "ChoiceParameterLinks",
            item(slots, 14)?,
            owner,
            names,
        )?)
        .child(super::export::values::choice_parameters_element(
            "ChoiceParameters",
            item(slots, 16)?,
            names,
        )?)
        .child(leaf(
            "QuickChoice",
            code_text(item(slots, 12)?, QUICK_CHOICE)?,
        ))
        .child(leaf(
            "ChoiceForm",
            reference_name(atom(item(slots, 11)?)?, names)?,
        ))
        .child(bool_element("ExtendedEdit", item(slots, 17)?)?)
        .child(leaf(
            "NameInDataSource",
            xml_text(string(item(record, 2)?)?),
        )))
}

/// Table/DimensionTable `fields` collection item:
/// `{1,{27,...},"<NameInDataSource>",<ReadOnly>,<AllowNull>}`.
fn field_properties(
    record: &[Brace],
    owner: Owner<'_>,
    context: &ExportContext,
) -> Result<Element> {
    let body = item(record, 1)?;
    let slots = list(body)?;
    if atom(item(slots, 0)?)? != "27" || slots.len() != 23 {
        bail!("Field: not an attribute body");
    }
    let typed = list(item(slots, 1)?)?;
    let head = header(item(typed, 1)?)?;
    let [name, synonym, comment] = header_elements(&head)?;
    let names = &context.names;
    Ok(el("Properties")
        .child(name)
        .child(synonym)
        .child(comment)
        .child(type_element("Type", item(typed, 2)?, names)?)
        .child(bool_element("PasswordMode", item(slots, 2)?)?)
        .child(localized_element("Format", item(slots, 3)?)?)
        .child(localized_element("EditFormat", item(slots, 18)?)?)
        .child(localized_element("ToolTip", item(slots, 4)?)?)
        .child(bool_element("MarkNegatives", item(slots, 5)?)?)
        .child(leaf("Mask", xml_text(string(item(slots, 6)?)?)))
        .child(bool_element("MultiLine", item(slots, 7)?)?)
        .child(bool_element("ExtendedEdit", item(slots, 17)?)?)
        .child(value_element("MinValue", item(slots, 8)?, names)?)
        .child(value_element("MaxValue", item(slots, 9)?, names)?)
        .child(bool_element("FillFromFillingValue", item(slots, 20)?)?)
        .child(value_element("FillValue", item(slots, 19)?, names)?)
        .child(leaf(
            "FillChecking",
            code_text(item(slots, 13)?, FILL_CHECKING)?,
        ))
        .child(super::export::values::choice_parameter_links_element(
            "ChoiceParameterLinks",
            item(slots, 14)?,
            owner,
            names,
        )?)
        .child(super::export::values::choice_parameters_element(
            "ChoiceParameters",
            item(slots, 16)?,
            names,
        )?)
        .child(leaf(
            "QuickChoice",
            code_text(item(slots, 12)?, QUICK_CHOICE)?,
        ))
        .child(leaf(
            "CreateOnInput",
            code_text(item(slots, 21)?, CREATE_ON_INPUT)?,
        ))
        .child(leaf(
            "ChoiceHistoryOnInput",
            code_text(item(slots, 22)?, CHOICE_HISTORY_ON_INPUT)?,
        ))
        .child(leaf(
            "ChoiceForm",
            reference_name(atom(item(slots, 11)?)?, names)?,
        ))
        .child(leaf(
            "NameInDataSource",
            xml_text(string(item(record, 2)?)?),
        ))
        .child(bool_element("ReadOnly", item(record, 3)?)?)
        .child(bool_element("AllowNull", item(record, 4)?)?))
}

/// Decode one child row into its XML element.
pub(crate) fn decode_row(kind: &str, row: &Brace, context: &ExportContext) -> Result<Element> {
    let parsed = Row::parse(row, kind)?;
    let head = parsed.header(kind)?;
    let full = full_name(&context.names, &head.uuid)?;
    let path = Path::from_full(kind, &full)?;
    let internal = internal_info(kind, &path, parsed.payload)?;
    let properties = match kind {
        schema::CUBE_KIND => cube_properties(&parsed, &head)?,
        schema::TABLE_KIND => table_properties(&parsed, &head, &full)?,
        schema::DIMENSION_TABLE_KIND => dimension_properties(&parsed, &head)?,
        other => bail!("unknown EDS child kind {other}"),
    };
    let children = child_objects(kind, &parsed, &head, &full, context)?;
    Ok(el(kind)
        .attr("uuid", &head.uuid)
        .child(internal)
        .child(properties)
        .child(children))
}

fn child_objects(
    kind: &str,
    row: &Row<'_>,
    head: &Header,
    full: &str,
    context: &ExportContext,
) -> Result<Element> {
    let mut objects = el("ChildObjects");
    let owner = Owner {
        kind,
        full_name: full,
    };
    match kind {
        schema::CUBE_KIND => {
            if row.has_collection(schema::CUBE_DIMENSION_TABLES_COLLECTION) {
                for entry in row.items(schema::CUBE_DIMENSION_TABLES_COLLECTION)? {
                    objects.children.push(leaf(
                        "DimensionTable",
                        short_name(&context.names, atom(entry)?)?,
                    ));
                }
            }
            if row.has_collection(schema::CUBE_INLINE_DIMENSIONS_COLLECTION) {
                for entry in row.items(schema::CUBE_INLINE_DIMENSIONS_COLLECTION)? {
                    let wrapper = list(entry)?;
                    let record = list(item(wrapper, 0)?)?;
                    let body = item(record, 1)?;
                    let typed = list(item(list(body)?, 1)?)?;
                    let head = header(item(typed, 1)?)?;
                    objects.children.push(
                        el("Dimension")
                            .attr("uuid", &head.uuid)
                            .child(inline_dimension_properties(record, owner, context)?),
                    );
                }
            }
            if row.has_collection(schema::CUBE_INLINE_RESOURCES_COLLECTION) {
                for entry in row.items(schema::CUBE_INLINE_RESOURCES_COLLECTION)? {
                    let wrapper = list(entry)?;
                    let record = list(item(wrapper, 0)?)?;
                    let body = item(record, 1)?;
                    let typed = list(item(list(body)?, 1)?)?;
                    let head = header(item(typed, 1)?)?;
                    objects.children.push(
                        el("Resource")
                            .attr("uuid", &head.uuid)
                            .child(inline_resource_properties(record, owner, context)?),
                    );
                }
            }
        }
        schema::TABLE_KIND | schema::DIMENSION_TABLE_KIND => {
            if row.has_collection(schema::FIELDS_COLLECTION) {
                for entry in row.items(schema::FIELDS_COLLECTION)? {
                    let wrapper = list(entry)?;
                    let record = list(item(wrapper, 0)?)?;
                    let body = item(record, 1)?;
                    let typed = list(item(list(body)?, 1)?)?;
                    let head = header(item(typed, 1)?)?;
                    objects.children.push(
                        el("Field")
                            .attr("uuid", &head.uuid)
                            .child(field_properties(record, owner, context)?),
                    );
                }
            }
        }
        other => bail!("unknown EDS child kind {other}"),
    }
    let _ = head;
    Ok(objects)
}

/// The name index contribution of an owned child row.
pub(crate) fn owned_names(kind: &str, row: &Brace, owner: &str) -> Result<ObjectNames> {
    let parsed = Row::parse(row, kind)?;
    let head = parsed.header(kind)?;
    if owner.is_empty() {
        bail!("{kind}: owner is required for names");
    }
    let full_name = format!("{owner}.{kind}.{}", head.name);
    let path = Path::from_full(kind, &full_name)?;
    let mut types = Vec::new();
    for category in schema::categories(kind) {
        let type_id = atom(item(parsed.payload, category.body_slot)?)?.to_string();
        let value_id = atom(item(parsed.payload, category.body_slot + 1)?)?.to_string();
        types.push(GeneratedTypeName {
            name: path.generated_name(kind, category.name_stem)?,
            category: category.category.to_string(),
            type_id,
            value_id,
        });
    }
    let children = inline_children(kind, &parsed, &full_name).unwrap_or_default();
    Ok(ObjectNames {
        uuid: head.uuid,
        full_name,
        children,
        types,
    })
}

fn inline_children(kind: &str, row: &Row<'_>, full_name: &str) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut add = |tag: &str, entry: &Brace| -> Result<()> {
        let wrapper = list(entry)?;
        let record = list(item(wrapper, 0)?)?;
        let body = item(record, 1)?;
        let typed = list(item(list(body)?, 1)?)?;
        let head = header(item(typed, 1)?)?;
        out.push((format!("{full_name}.{tag}.{}", head.name), head.uuid));
        Ok(())
    };
    match kind {
        schema::CUBE_KIND => {
            if row.has_collection(schema::CUBE_INLINE_DIMENSIONS_COLLECTION) {
                for entry in row.items(schema::CUBE_INLINE_DIMENSIONS_COLLECTION)? {
                    add("Dimension", entry)?;
                }
            }
            if row.has_collection(schema::CUBE_INLINE_RESOURCES_COLLECTION) {
                for entry in row.items(schema::CUBE_INLINE_RESOURCES_COLLECTION)? {
                    add("Resource", entry)?;
                }
            }
        }
        schema::TABLE_KIND | schema::DIMENSION_TABLE_KIND => {
            for entry in row.items(schema::FIELDS_COLLECTION).unwrap_or_default() {
                add("Field", entry)?;
            }
        }
        _ => {}
    }
    Ok(out)
}

/// The cube and table UUIDs a root ExternalDataSource row lists.
pub(crate) fn root_children(text: &str) -> Result<(Vec<String>, Vec<String>)> {
    let row = parse_row(text.as_bytes())?;
    let root = list(&row)?;
    let count = usize::try_from(super::export::number(item(root, 2)?)?)
        .map_err(|_| anyhow!("root collection count"))?;
    let mut cubes = Vec::new();
    let mut tables = Vec::new();
    for collection in root.iter().skip(3).take(count) {
        let fields = list(collection)?;
        let id = atom(item(fields, 0)?)?;
        let count = atom(item(fields, 1)?)?
            .parse::<usize>()
            .map_err(|_| anyhow!("root child count"))?;
        let members = fields
            .get(2..2 + count)
            .ok_or_else(|| anyhow!("root child collection is short"))?;
        let target = match id {
            id if id == schema::ROOT_CUBES_COLLECTION => &mut cubes,
            id if id == schema::ROOT_TABLES_COLLECTION => &mut tables,
            _ => continue,
        };
        for member in members {
            target.push(atom(member)?.to_string());
        }
    }
    Ok((cubes, tables))
}

/// The dimension-table UUIDs a Cube row lists.
pub(crate) fn dimension_table_refs(text: &str) -> Result<Vec<String>> {
    let row = parse_row(text.as_bytes())?;
    let parsed = Row::parse(&row, schema::CUBE_KIND)?;
    parsed
        .items(schema::CUBE_DIMENSION_TABLES_COLLECTION)?
        .iter()
        .map(|member| Ok(atom(member)?.to_string()))
        .collect()
}

/// Recognise a child descriptor by its measured body layout. `None` is not an
/// error: the caller keeps its previous fallbacks.
pub(crate) fn family(text: &str, uuid: &str) -> Option<(&'static str, &'static str)> {
    let row = parse_row(text.as_bytes()).ok()?;
    let parsed = Row::parse(&row, schema::CUBE_KIND)
        .ok()
        .or_else(|| Row::parse(&row, schema::DIMENSION_TABLE_KIND).ok())
        .or_else(|| Row::parse(&row, schema::TABLE_KIND).ok())?;
    let kind = parsed_kind(parsed.payload).ok()?;
    if parsed.header(kind).ok()?.uuid != uuid {
        return None;
    }
    Some((kind, schema::collection(kind)))
}

fn parsed_kind(payload: &[Brace]) -> Result<&'static str> {
    match (atom(item(payload, 0)?)?, payload.len()) {
        ("0", schema::CUBE_BODY_ARITY) => Ok(schema::CUBE_KIND),
        ("0", schema::DIMENSION_TABLE_BODY_ARITY) => Ok(schema::DIMENSION_TABLE_KIND),
        ("11", schema::TABLE_BODY_ARITY) => Ok(schema::TABLE_KIND),
        (code, len) => bail!("not an EDS child: code {code}, arity {len}"),
    }
}

/// Whether the row has a measured EDS-child body shape but failed `family`.
/// The caller refuses it instead of letting it fall into `IntegrationService`.
pub(crate) fn looks_like_eds_child(text: &str) -> bool {
    let Ok(row) = parse_row(text.as_bytes()) else {
        return false;
    };
    let Ok(root) = list(&row) else {
        return false;
    };
    let Ok(payload) = item(root, 1) else {
        return false;
    };
    let Some(payload) = payload.as_list() else {
        return false;
    };
    matches!(
        (payload.first().and_then(Brace::as_atom), payload.len()),
        (Some("0"), schema::CUBE_BODY_ARITY)
            | (Some("0"), schema::DIMENSION_TABLE_BODY_ARITY)
            | (Some("11"), schema::TABLE_BODY_ARITY)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ServiceDesk 3.10.28 / 8.5.1.1529 row, independently documented by
    // `feat/rea-form-property-inventory-20261022`
    // (`docs/evidence/rea-external-data-source-20261022/README.md`). It has
    // non-empty KeyFields/InputByString and both N and S field types.
    const ROW: &str =
        include_str!("../../tests/fixtures/eds-branch/external_data_source_table_row.txt");

    #[test]
    fn branch_table_row_names_key_fields_and_input_by_string() {
        let row = parse_row(ROW.as_bytes()).unwrap();
        let parsed = Row::parse(&row, schema::TABLE_KIND).unwrap();
        let (key_fields, input_by_string) = table_ref_lists(&parsed).unwrap();
        assert_eq!(key_fields, vec!["product_id".to_string()]);
        assert_eq!(input_by_string, vec!["product_id".to_string()]);

        let names = field_names(&parsed).unwrap();
        assert_eq!(
            names
                .get("a9a45781-332e-4c72-ac04-579e8e11e6c5")
                .map(String::as_str),
            Some("product_id")
        );
        assert_eq!(
            names
                .get("b35dfac3-6d64-4c5f-8feb-2758776bec94")
                .map(String::as_str),
            Some("_c_id")
        );
    }

    #[test]
    fn branch_table_row_field_types_are_n_and_s() {
        let row = parse_row(ROW.as_bytes()).unwrap();
        let parsed = Row::parse(&row, schema::TABLE_KIND).unwrap();
        let mut patterns = Vec::new();
        for entry in parsed.items(schema::FIELDS_COLLECTION).unwrap() {
            let wrapper = list(entry).unwrap();
            let record = list(item(wrapper, 0).unwrap()).unwrap();
            let body = item(record, 1).unwrap();
            let typed = list(item(list(body).unwrap(), 1).unwrap()).unwrap();
            let pattern = list(item(typed, 2).unwrap()).unwrap();
            patterns.push(
                pattern[1].as_list().unwrap()[0]
                    .as_str()
                    .unwrap()
                    .to_string(),
            );
        }
        assert_eq!(patterns, ["N", "S"]);
    }
}
