//! Disposable public fixtures shared by portable MCP payload and Unix SDK tests.
#![allow(dead_code)]
#[path = "selected_export_fixture.rs"]
mod selected;
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    InvalidDescriptor,
    OpaqueForm,
    NestedInvalid,
    LegacyCommand,
    CommonForm,
}
pub struct Case {
    pub root: PathBuf,
    pub database: PathBuf,
    pub rows: PathBuf,
    pub source: PathBuf,
    pub settings: PathBuf,
    pub object: String,
    pub rows_dir: bool,
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
pub fn tree(path: &Path) -> BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            (
                e.path()
                    .strip_prefix(path)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}
fn unpack(path: &Path) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(fs::File::open(path).unwrap())
        .read_to_end(&mut out)
        .unwrap();
    out
}
fn set_form_body(database: &Path) {
    let mut reader = ibcmd_filedb::FileDb::open(database).unwrap();
    let mut rows = reader.config_rows("CONFIG").unwrap();
    drop(reader);
    let row = rows
        .iter_mut()
        .find(|r| r.file_name == "aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaa1.0")
        .unwrap();
    let mut packed = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    packed
        .write_all(b"{4,{0},\"Procedure KeepMe()\r\nEndProcedure\r\n\",{0}}")
        .unwrap();
    row.binary = packed.finish().unwrap();
    row.data_size = row.binary.len() as u64;
    ibcmd_filedb::replace_config_table(database, "CONFIG", &rows).unwrap();
}
impl Case {
    pub fn new(scenario: Scenario, rows_dir: bool) -> Self {
        use ibcmd_rs::metadata_model::brace::Brace;
        let root =
            std::env::temp_dir().join(format!("mcp-export-completeness-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("db")).unwrap();
        let database = root.join("db/1Cv8.1CD");
        fs::write(&database, unpack(&fixture("file-infobase/tiny.1CD.gz"))).unwrap();
        let object = match scenario {
            Scenario::InvalidDescriptor => {
                selected::install_unadmitted_common_module_descriptor(&database);
                "CommonModule.ФайловаяБазаПроверка".into()
            }
            Scenario::OpaqueForm | Scenario::CommonForm => {
                selected::install_unadmitted_form_asset(&database, true);
                if matches!(scenario, Scenario::CommonForm) {
                    set_form_body(&database);
                }
                "CommonForm.OpaqueForm".into()
            }
            Scenario::LegacyCommand => {
                selected::install_legacy_fallback_object(&database, true, None)
            }
            Scenario::NestedInvalid => {
                let recorded = root.join("recorded.1CD");
                fs::write(
                    &recorded,
                    unpack(&fixture("native-export-commands/owned/database.1CD.gz")),
                )
                .unwrap();
                let mut reader = ibcmd_filedb::FileDb::open(&recorded).unwrap();
                let rows = reader.config_rows("CONFIG").unwrap();
                drop(reader);
                ibcmd_filedb::replace_config_table(&database, "CONFIG", &rows).unwrap();
                selected::mutate_descriptor(
                    &database,
                    "c099c30b-8a08-45b0-be7d-9594366e78a5",
                    |row| {
                        let wrapper = row.as_list_mut().unwrap()[1].as_list_mut().unwrap();
                        let form = wrapper[1].as_list_mut().unwrap();
                        assert_eq!(form[0].as_atom(), Some("13"));
                        assert_eq!(form[2].as_atom(), Some("0"));
                        form[2] = Brace::num(99);
                    },
                );
                "Catalog.CorpusList.Form.ListForm".into()
            }
        };
        let rows = root.join("rows");
        fs::create_dir(&rows).unwrap();
        let mut reader = ibcmd_filedb::FileDb::open(&database).unwrap();
        for row in reader.config_rows("CONFIG").unwrap() {
            fs::write(
                rows.join(format!("{}__part{}.bin", row.file_name, row.part_no)),
                row.binary,
            )
            .unwrap();
        }
        drop(reader);
        let source = root.join("source");
        fs::create_dir(&source).unwrap();
        for (name, bytes) in tree(&fixture("file-infobase/tiny-xml")) {
            let p = source.join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, bytes).unwrap();
        }
        for path in [
            "CommonForms/OpaqueForm.xml",
            "CommonForms/OpaqueForm/Ext/Form.xml",
            "Catalogs/CorpusList/Forms/ListForm.xml",
        ] {
            let p = source.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, b"local selected sentinel").unwrap();
        }
        fs::write(source.join("sentinel.txt"), b"unrelated local sentinel").unwrap();
        let settings = root.join("mcp.toml");
        let backend = if rows_dir {
            "rows-dir='rows'"
        } else {
            "db-path='db'"
        };
        fs::write(&settings,format!("[[database]]\nname='demo'\nplatform='8.5.1'\n{backend}\nagent-write=false\n[mcp]\ndatabase='demo'\nsource-root='source'\n")).unwrap();
        Self {
            root,
            database,
            rows,
            source,
            settings,
            object,
            rows_dir,
        }
    }
    pub fn input(&self) -> BTreeMap<String, Vec<u8>> {
        tree(&self.root)
    }
}
