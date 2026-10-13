//! A new external data processor or report (.epf/.erf) built from its XML
//! tree alone (issue #438): the compiled load needs a package to start from,
//! so a platform-built one is taken as the seed and given the tree's own
//! identities -- the root uuid, its contained ObjectId, its generated type and
//! value ids and its name. The load then carries the whole tree onto it: the
//! root's properties and child objects replace the seed's, the tree's forms,
//! templates and modules are added, and the seed's own are removed.
//!
//! The seeds are the platform-built fixtures `tests/fixtures/external/
//! variants/no_module/input.epf` and `tests/fixtures/external/test_report/
//! input.erf` (8.3.27.2214); their identities are those their own XML dumps.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};

use super::compiled;
use crate::module_blob::{deflate_raw, inflate_raw};

/// One seed package and the identities its XML dump names.
struct Seed {
    bytes: &'static [u8],
    tag: &'static str,
    uuid: &'static str,
    object_id: &'static str,
    type_id: &'static str,
    value_id: &'static str,
    name: &'static str,
    /// The seed's own child objects: renamed out of the way, so a tree child
    /// of the same name is a new object, never an edit of the seed's.
    children: &'static [&'static str],
}

const PROCESSOR: Seed = Seed {
    bytes: include_bytes!("../../tests/fixtures/external/variants/no_module/input.epf"),
    tag: "ExternalDataProcessor",
    uuid: "dc0e1582-2d76-4880-ba01-3711da6f4448",
    object_id: "80ba9317-3805-45ad-950d-e4cab1dd642b",
    type_id: "87f78a5f-837e-4f96-995e-37ab22e02b56",
    value_id: "0e90fd7d-d3cd-4e67-a2c1-8410e4ea602e",
    name: "ТестОбработка",
    children: &["Форма", "Макет", "Строка", "Строки", "Число"],
};

const REPORT: Seed = Seed {
    bytes: include_bytes!("../../tests/fixtures/external/test_report/input.erf"),
    tag: "ExternalReport",
    uuid: "b0509899-141e-4cda-ba24-de8ed9711646",
    object_id: "3ae658fc-eb5f-4e2d-a3b9-8f021f275ff7",
    type_id: "1809b0db-0610-4885-86d0-4fcab351518e",
    value_id: "c5c17663-e8dc-4c06-9f35-d58aa343b6d2",
    name: "ТестОтчет",
    children: &["ФормаОтчета", "ОсновнаяСхемаКомпоновкиДанных"],
};

/// `copyinfo` of an object that references no configuration object.
const EMPTY_COPYINFO: &str = "\u{feff}{4,\r\n{0},\r\n{0},\r\n{0},\r\n{0,0},\r\n{0}\r\n}";

/// Kinds of configuration object a tree may name: `<Kind>.<Name>` in an XML
/// value or a help link; `cfg:<Kind>Ref…` in a type.
const CONFIGURATION_KINDS: [&str; 24] = [
    "Catalog",
    "Document",
    "Enum",
    "ChartOfCharacteristicTypes",
    "ChartOfAccounts",
    "ChartOfCalculationTypes",
    "ExchangePlan",
    "BusinessProcess",
    "Task",
    "InformationRegister",
    "AccumulationRegister",
    "AccountingRegister",
    "CalculationRegister",
    "Constant",
    "CommonForm",
    "CommonPicture",
    "CommonTemplate",
    "CommonCommand",
    "CommonModule",
    "StyleItem",
    "Style",
    "SettingsStorage",
    "DefinedType",
    "FilterCriterion",
];

/// The end of a dotted identifier starting at `start`.
fn identifier_end(text: &str, start: usize) -> usize {
    text[start..]
        .find(|c: char| !(c.is_alphanumeric() || c == '.' || c == '_'))
        .map_or(text.len(), |len| start + len)
}

/// The configuration objects `text` names.
fn references_in(text: &str, found: &mut BTreeSet<String>) {
    let mut at = 0;
    while let Some(offset) = text[at..].find("cfg:") {
        let start = at + offset + 4;
        let end = identifier_end(text, start);
        let name = &text[start..end];
        if !name.is_empty() && !name.starts_with("External") {
            found.insert(format!("cfg:{name}"));
        }
        at = end.max(start);
    }
    for kind in CONFIGURATION_KINDS {
        for opener in [">", "\"", "/"] {
            let needle = format!("{opener}{kind}.");
            let mut at = 0;
            while let Some(offset) = text[at..].find(&needle) {
                let start = at + offset + opener.len();
                let end = identifier_end(text, start);
                found.insert(text[start..end].trim_end_matches('.').to_owned());
                at = end.max(start + 1);
            }
        }
    }
}

/// The configuration objects the tree names: without the configuration
/// their identities are unknown -- a platform building the object from its
/// XML in an empty infobase drops them -- so such a build is refused.
fn configuration_references(tree: &Path) -> Result<Vec<String>> {
    let mut found = BTreeSet::new();
    let mut stack = vec![tree.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in
            fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))?
        {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|ext| ext == "xml" || ext == "html")
            {
                references_in(&fs::read_to_string(&path).unwrap_or_default(), &mut found);
            }
        }
    }
    Ok(found.into_iter().collect())
}

/// The identities of the tree's external root.
#[derive(Debug, PartialEq, Eq)]
struct Identity {
    uuid: String,
    object_id: String,
    type_id: String,
    value_id: String,
    name: String,
}

/// The text between `open` and the next `close` after `from`.
fn between<'a>(text: &'a str, from: usize, open: &str, close: &str) -> Option<(&'a str, usize)> {
    let start = from + text[from..].find(open)? + open.len();
    let end = start + text[start..].find(close)?;
    Some((&text[start..end], end))
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

/// The root's identities as its XML states them; the `Object` generated type
/// is the one the main row carries.
fn identity(root_xml: &str, tag: &str) -> Result<Identity> {
    let opening = format!("<{tag} uuid=\"");
    let (uuid, at) = between(root_xml, 0, &opening, "\"")
        .with_context(|| format!("the root XML has no <{tag} uuid=…>"))?;
    let (object_id, _) = between(root_xml, at, "<xr:ObjectId>", "</xr:ObjectId>")
        .context("the root XML names no contained ObjectId")?;
    let object_at = root_xml[at..]
        .find("category=\"Object\"")
        .map(|found| at + found)
        .context("the root XML names no Object generated type")?;
    let (type_id, after) = between(root_xml, object_at, "<xr:TypeId>", "</xr:TypeId>")
        .context("the Object generated type has no TypeId")?;
    let (value_id, _) = between(root_xml, after, "<xr:ValueId>", "</xr:ValueId>")
        .context("the Object generated type has no ValueId")?;
    let (name, _) =
        between(root_xml, at, "<Name>", "</Name>").context("the root XML has no <Name>")?;
    let identity = Identity {
        uuid: uuid.to_ascii_lowercase(),
        object_id: object_id.trim().to_ascii_lowercase(),
        type_id: type_id.trim().to_ascii_lowercase(),
        value_id: value_id.trim().to_ascii_lowercase(),
        name: name.trim().to_owned(),
    };
    for id in [
        &identity.uuid,
        &identity.object_id,
        &identity.type_id,
        &identity.value_id,
    ] {
        if !is_uuid(id) {
            bail!("the root XML identity `{id}` is not a uuid");
        }
    }
    if !crate::external::header::is_object_name(&identity.name) {
        bail!("the root name `{}` is not an identifier", identity.name);
    }
    Ok(identity)
}

/// The external root XML of `tree` (a directory, or the root file itself)
/// with its seed: `None` when the tree is not an external object.
fn external_root(tree: &Path) -> Result<Option<(PathBuf, String, &'static Seed)>> {
    let candidates = if tree.is_file() {
        vec![tree.to_path_buf()]
    } else {
        let mut found = fs::read_dir(tree)
            .with_context(|| format!("failed to read {}", tree.display()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "xml"))
            .collect::<Vec<_>>();
        found.sort();
        found
    };
    let mut roots = Vec::new();
    for path in candidates {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        for seed in [&PROCESSOR, &REPORT] {
            if text.contains(&format!("<{} uuid=\"", seed.tag)) {
                roots.push((path.clone(), text.clone(), seed));
            }
        }
    }
    match roots.len() {
        0 => Ok(None),
        1 => Ok(roots.pop()),
        _ => bail!("the tree holds more than one external root XML"),
    }
}

/// `text` with every seed identity replaced by the tree's.
fn rekeyed(text: &str, seed: &Seed, target: &Identity, children: &[(String, String)]) -> String {
    let mut out = text.to_owned();
    for (from, to) in children {
        out = out.replace(from.as_str(), to.as_str());
    }
    for (from, to) in [
        (seed.uuid, target.uuid.as_str()),
        (seed.object_id, target.object_id.as_str()),
        (seed.type_id, target.type_id.as_str()),
        (seed.value_id, target.value_id.as_str()),
    ] {
        out = out.replace(from, to);
    }
    out = out.replace(
        &format!("\"{}\"", seed.name),
        &format!("\"{}\"", target.name),
    );
    for (index, child) in seed.children.iter().enumerate() {
        out = out.replace(
            &format!("\"{child}\""),
            &format!("\"ibcmdSeedChild{index}\""),
        );
    }
    out
}

/// Writes the seed package, given the tree's identities, to `path`.
fn write_seed(seed: &Seed, target: &Identity, path: &Path) -> Result<()> {
    let raw = path.with_extension("raw");
    fs::write(&raw, seed.bytes).context("failed to write the seed package")?;
    let (archive, limits) = compiled::decode_base(&raw)?;
    // The seed's own forms and templates get fresh ids too: a tree child
    // with the very id of a seed child (a tree made from the seed's own
    // source) would otherwise be removed with it.
    let children = archive
        .image()
        .entries()
        .iter()
        .filter_map(|entry| {
            let key = entry.logical_key().as_str();
            let id = key.split('.').next()?;
            (is_uuid(id) && ![seed.uuid, seed.object_id, seed.type_id, seed.value_id].contains(&id))
                .then(|| id.to_owned())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|id| {
            let fresh =
                crate::external::derived_uuid(&format!("ibcmd-rs seed child {id} {}", target.uuid));
            (id, fresh)
        })
        .collect::<Vec<_>>();
    let mut rows = BTreeMap::new();
    let mut dropped = BTreeSet::new();
    for entry in archive.image().entries() {
        let key = entry.logical_key().as_str();
        let new_key = rekeyed(key, seed, target, &children);
        let packed = entry.packed_payload();
        let rewritten = match inflate_raw(packed)
            .ok()
            .and_then(|plain| String::from_utf8(plain).ok())
        {
            Some(text) => {
                let new_text = rekeyed(&text, seed, target, &children);
                (new_text != text)
                    .then(|| deflate_raw(new_text.as_bytes()))
                    .transpose()?
            }
            None => None,
        };
        if new_key != key {
            dropped.insert(key.to_owned());
            rows.insert(new_key, rewritten.unwrap_or_else(|| packed.to_vec()));
        } else if let Some(rewritten) = rewritten {
            rows.insert(new_key, rewritten);
        }
    }
    // The seed's copyinfo names the seed's own references; the built object
    // refers to nothing outside itself (`configuration_references` refused
    // the rest), and 8.3.27.2214 building such an object from its XML in an
    // empty infobase writes this one (`/LoadExternalDataProcessorOrReportFromFiles`).
    rows.insert(
        "copyinfo".to_owned(),
        deflate_raw(EMPTY_COPYINFO.as_bytes())?,
    );
    compiled::write_edit(&archive, rows, &dropped, path, limits)?;
    let _ = fs::remove_file(&raw);
    Ok(())
}

/// Builds `output` from the external tree `tree` with no base package:
/// `Ok(None)` when the tree is not an external object.
pub fn build_external(
    tree: &Path,
    output: &Path,
    source_version: crate::legacy_version::InfobaseConfigSourceVersion,
) -> std::result::Result<Option<super::LoadReport>, super::LoadError> {
    let Some((root_xml, text, seed)) = external_root(tree)? else {
        return Ok(None);
    };
    let target = identity(&text, seed.tag)?;
    let references = configuration_references(if tree.is_file() {
        root_xml.parent().unwrap_or(tree)
    } else {
        tree
    })?;
    if !references.is_empty() {
        return Err(anyhow!(
            "the object refers to configuration objects, whose identities only the \
             configuration holds -- build it with cf load --base <a file of that \
             configuration's object>: {}",
            references.join(", ")
        )
        .into());
    }
    let tree_dir = if tree.is_file() {
        root_xml
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow!("the root XML has no folder"))?
    } else {
        tree.to_path_buf()
    };
    let scratch = std::env::temp_dir().join(format!(
        "ibcmd-rs-seed-{}-{}",
        std::process::id(),
        target.uuid
    ));
    fs::create_dir_all(&scratch).context("failed to create the seed scratch folder")?;
    let seed_path = scratch.join(if seed.tag == "ExternalReport" {
        "seed.erf"
    } else {
        "seed.epf"
    });
    crate::compiler::bodies::form_native::record_data_path_steps();
    let result = write_seed(seed, &target, &seed_path)
        .map_err(super::LoadError::from)
        .and_then(|()| {
            match super::load_onto_base(&tree_dir, &seed_path, output, source_version) {
                // The tree is the seed itself: the rekeyed seed is the package.
                Err(super::LoadError::NoChanges) => fs::copy(&seed_path, output)
                    .map(|_| super::LoadReport::default())
                    .context("failed to write the package")
                    .map_err(super::LoadError::from),
                other => other,
            }
        });
    let steps = crate::compiler::bodies::form_native::take_data_path_steps();
    let result = result.and_then(|report| {
        let copyinfo = copyinfo_with_steps(&text, seed, &target, &steps)?;
        if copyinfo != EMPTY_COPYINFO {
            replace_entry(output, "copyinfo", copyinfo.as_bytes(), &scratch)?;
        }
        Ok(report)
    });
    let _ = fs::remove_dir_all(&scratch);
    result.map(Some)
}

/// Builtin types whose members a form path steps through by column id.
const BUILTIN_TYPES: [(&str, &str); 2] = [
    ("v8:ValueTable", "acf6192e-81ca-46ef-93a6-5a6968b78663"),
    ("v8:ValueTree", "e603c0f2-92fb-4d47-8f38-a44a381cf235"),
];

/// The generated `TabularSection` type id of each tabular section the root
/// XML declares.
fn tabular_section_types(root_xml: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut at = 0;
    while let Some(offset) = root_xml[at..].find("<TabularSection uuid=\"") {
        let start = at + offset;
        let end = root_xml[start..]
            .find("</TabularSection>")
            .map_or(root_xml.len(), |len| start + len);
        let body = &root_xml[start..end];
        let type_id = body.find("category=\"TabularSection\"").and_then(|found| {
            between(body, found, "<xr:TypeId>", "</xr:TypeId>").map(|(id, _)| id.trim())
        });
        let name = between(body, 0, "<Properties>", "</Properties>")
            .and_then(|(properties, _)| between(properties, 0, "<Name>", "</Name>"))
            .map(|(name, _)| name.trim());
        if let (Some(type_id), Some(name)) = (type_id, name) {
            out.insert(name.to_owned(), type_id.to_ascii_lowercase());
        }
        at = end;
    }
    out
}

/// `copyinfo` naming the members the forms' data paths step through.
///
/// 8.3.27.2214 keeps a form `<DataPath>` only if each step it takes into a
/// type is listed in the third block as `{<segment>,<type id>,{<n>|-1}}`:
/// with the platform's own copyinfo of a built processor its `Объект.Адрес`
/// paths survive, with that block removed both are dropped, and the other
/// blocks change nothing (corpus processor `узОбновлениеПрограммы`). The
/// steps are what the form compiler walked
/// (`form_native::take_data_path_steps`): through the object's own type and
/// its tabular sections (107 of the 111 corpus processors' blocks predicted
/// so), and through value-table and value-tree columns.
fn copyinfo_with_steps(
    root_xml: &str,
    seed: &Seed,
    target: &Identity,
    steps: &BTreeSet<(String, String)>,
) -> Result<String> {
    let internal = if seed.tag == "ExternalReport" {
        "Report"
    } else {
        "DataProcessor"
    };
    let own = format!("meta|{internal}.{}|", target.name);
    let sections = tabular_section_types(root_xml);
    let mut entries = Vec::new();
    for (segment, parent) in steps {
        let type_id = if let Some(section) = parent.strip_prefix(&own) {
            if section.is_empty() {
                target.type_id.clone()
            } else {
                match sections.get(section) {
                    Some(type_id) => type_id.clone(),
                    None => continue,
                }
            }
        } else if let Some((_, uuid)) = BUILTIN_TYPES.iter().find(|(name, _)| name == parent) {
            (*uuid).to_owned()
        } else {
            continue;
        };
        let index = segment
            .trim_start_matches('{')
            .trim_end_matches('}')
            .parse::<i64>()
            .map_or_else(|_| "-1".to_owned(), |number| number.to_string());
        entries.push(format!("{{\r\n{segment},{type_id},\r\n{{{index}}}\r\n}}"));
    }
    if entries.is_empty() {
        return Ok(EMPTY_COPYINFO.to_owned());
    }
    Ok(format!(
        "\u{feff}{{4,\r\n{{0}},\r\n{{0}},\r\n{{{},\r\n{}\r\n}},\r\n{{0,0}},\r\n{{0}}\r\n}}",
        entries.len(),
        entries.join(",\r\n")
    ))
}

/// `file` with the entry `key` holding `plain`.
fn replace_entry(file: &Path, key: &str, plain: &[u8], scratch: &Path) -> Result<()> {
    let rewritten = scratch.join("rewritten");
    {
        let (archive, limits) = compiled::decode_base(file)?;
        let rows = BTreeMap::from([(key.to_owned(), deflate_raw(plain)?)]);
        compiled::write_edit(&archive, rows, &BTreeSet::new(), &rewritten, limits)?;
    }
    fs::copy(&rewritten, file).context("failed to rewrite the package")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_root_identity() {
        let xml = "<ExternalDataProcessor uuid=\"DC0E1582-2D76-4880-BA01-3711DA6F4448\">\
            <InternalInfo><xr:ContainedObject><xr:ClassId>c</xr:ClassId>\
            <xr:ObjectId>80ba9317-3805-45ad-950d-e4cab1dd642b</xr:ObjectId></xr:ContainedObject>\
            <xr:GeneratedType name=\"ExternalDataProcessorObject.Б\" category=\"Object\">\
            <xr:TypeId>87f78a5f-837e-4f96-995e-37ab22e02b56</xr:TypeId>\
            <xr:ValueId>0e90fd7d-d3cd-4e67-a2c1-8410e4ea602e</xr:ValueId></xr:GeneratedType>\
            </InternalInfo><Properties><Name>Б</Name></Properties>";
        let found = identity(xml, "ExternalDataProcessor").unwrap();
        assert_eq!(found.uuid, "dc0e1582-2d76-4880-ba01-3711da6f4448");
        assert_eq!(found.name, "Б");
        assert!(
            identity(
                &xml.replace("<Name>Б", "<Name>1 Б"),
                "ExternalDataProcessor"
            )
            .is_err()
        );
    }

    #[test]
    fn rekeys_ids_and_the_quoted_name_only() {
        let target = Identity {
            uuid: "11111111-1111-1111-1111-111111111111".into(),
            object_id: "22222222-2222-2222-2222-222222222222".into(),
            type_id: "33333333-3333-3333-3333-333333333333".into(),
            value_id: "44444444-4444-4444-4444-444444444444".into(),
            name: "Новая".into(),
        };
        let text = format!(
            "{{{},{},\"ТестОбработка\",ТестОбработкаX}}",
            PROCESSOR.uuid, PROCESSOR.object_id
        );
        assert_eq!(
            rekeyed(&text, &PROCESSOR, &target, &[]),
            "{11111111-1111-1111-1111-111111111111,22222222-2222-2222-2222-222222222222,\
             \"Новая\",ТестОбработкаX}"
        );
    }

    fn target() -> Identity {
        Identity {
            uuid: "11111111-1111-1111-1111-111111111111".into(),
            object_id: "22222222-2222-2222-2222-222222222222".into(),
            type_id: "33333333-3333-3333-3333-333333333333".into(),
            value_id: "44444444-4444-4444-4444-444444444444".into(),
            name: "Б".into(),
        }
    }

    #[test]
    fn copyinfo_lists_own_and_value_table_steps_only() {
        let root = "<TabularSection uuid=\"t\"><InternalInfo>\
            <xr:GeneratedType name=\"x\" category=\"TabularSection\">\
            <xr:TypeId>55555555-5555-5555-5555-555555555555</xr:TypeId></xr:GeneratedType>\
            </InternalInfo><Properties><Name>ТЧ</Name></Properties></TabularSection>";
        let steps = BTreeSet::from([
            ("{0,a}".to_owned(), "meta|DataProcessor.Б|".to_owned()),
            ("{0,c}".to_owned(), "meta|DataProcessor.Б|ТЧ".to_owned()),
            ("{2}".to_owned(), "v8:ValueTable".to_owned()),
            ("{0,f}".to_owned(), "meta|Catalog.Чужой|".to_owned()),
        ]);
        let text = copyinfo_with_steps(root, &PROCESSOR, &target(), &steps).unwrap();
        assert!(text.contains("{3,"));
        assert!(text.contains("{0,a},33333333-3333-3333-3333-333333333333,\r\n{-1}"));
        assert!(text.contains("{0,c},55555555-5555-5555-5555-555555555555,\r\n{-1}"));
        assert!(text.contains("{2},acf6192e-81ca-46ef-93a6-5a6968b78663,\r\n{2}"));
        assert!(!text.contains("{0,f}"));
        assert_eq!(
            copyinfo_with_steps(root, &PROCESSOR, &target(), &BTreeSet::new()).unwrap(),
            EMPTY_COPYINFO
        );
    }

    #[test]
    fn finds_configuration_references() {
        let mut found = BTreeSet::new();
        references_in(
            "<v8:Type>cfg:CatalogRef.Валюты</v8:Type><v8:Type>cfg:ExternalDataProcessorObject.Б\
             </v8:Type><xr:Ref>CommonPicture.Лого</xr:Ref><a href=\"Catalog.Банки/Help\">\
             <xr:Ref>StdPicture.Print</xr:Ref>",
            &mut found,
        );
        assert_eq!(
            found.into_iter().collect::<Vec<_>>(),
            [
                "Catalog.Банки",
                "CommonPicture.Лого",
                "cfg:CatalogRef.Валюты"
            ]
        );
    }

    #[test]
    fn builds_a_processor_from_its_tree_alone() {
        let scratch = std::env::temp_dir().join(format!("ibcmd-seed-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&scratch);
        let tree = scratch.join("tree");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/external/test_processor/input.epf");
        super::super::export_base(
            &fixture,
            &tree,
            crate::legacy_version::InfobaseConfigSourceVersion::V2_20,
        )
        .unwrap();
        let output = scratch.join("out.epf");
        let version = crate::legacy_version::InfobaseConfigSourceVersion::V2_20;
        build_external(&tree, &output, version).unwrap().unwrap();
        let back = scratch.join("back");
        super::super::export_base(&output, &back, version).unwrap();
        let read = |root: &Path, name: &str| fs::read(root.join(name)).unwrap();
        assert_eq!(
            read(&tree, "ТестОбработка.xml"),
            read(&back, "ТестОбработка.xml")
        );
        let _ = fs::remove_dir_all(&scratch);
    }
}
