//! Clean-room 8.3.27.2214 file snapshots; no platform is launched.
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output},
};
struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let base = std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-fixtures"));
        fs::create_dir_all(&base).unwrap();
        let path = base.join(format!(
            "ibcmd-file-extension-{tag}-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn db(&self, fixture: &str) -> PathBuf {
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(
            fs::File::open(fixtures().join(format!("{fixture}.1CD.gz"))).unwrap(),
        )
        .read_to_end(&mut bytes)
        .unwrap();
        let db = self.0.join("db");
        fs::create_dir(&db).unwrap();
        fs::write(db.join("1Cv8.1CD"), bytes).unwrap();
        db
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if std::env::var_os("IBCMD_TEST_KEEP_FIXTURES").as_deref()
            != Some(std::ffi::OsStr::new("1"))
        {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/file-extension-export")
}
fn export(db: &Path, name: &str, out: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ibcmd-rs"))
        .args(["infobase", "config", "export"])
        .arg("--db-path")
        .arg(db)
        .arg("--extension")
        .arg(name)
        .arg(out)
        .env("PATH", "")
        .output()
        .unwrap()
}
fn hashes(root: &Path) -> BTreeMap<String, String> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            (
                e.path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                format!("{:x}", Sha256::digest(fs::read(e.path()).unwrap())),
            )
        })
        .collect()
}
fn parity(fixture: &str, name: &str, expected: &str) {
    let dir = Scratch::new(expected);
    let db = dir.db(fixture);
    let out = dir.0.join("output");
    let before = fs::read(db.join("1Cv8.1CD")).unwrap();
    let run = export(&db, name, &out);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let expected: BTreeMap<String, String> = serde_json::from_slice(
        &fs::read(fixtures().join(format!("{expected}-sha256.json"))).unwrap(),
    )
    .unwrap();
    assert!(expected.contains_key("ConfigDumpInfo.xml"));
    assert_eq!(hashes(&out), expected);
    assert_eq!(fs::read(db.join("1Cv8.1CD")).unwrap(), before);
    assert!(!ibcmd_filedb::backup_file(&db).exists());
}
#[test]
fn own_extension_exact_native_xml_from_two_extensions() {
    parity("two-extensions", "ТестРасширение", "own");
}
#[test]
fn second_extension_has_only_its_native_tree() {
    parity("two-extensions", "Second520", "second");
}
#[test]
fn borrowed_common_module_exact_native_xml() {
    parity("borrowed", "ТестРасширение", "borrowed");
}
#[test]
fn unknown_name_is_code_one_without_output() {
    let dir = Scratch::new("unknown");
    let db = dir.db("two-extensions");
    let out = dir.0.join("output");
    let before = fs::read(db.join("1Cv8.1CD")).unwrap();
    let run = export(&db, "Missing520", &out);
    assert_eq!(run.status.code(), Some(1));
    assert!(!out.exists());
    assert!(String::from_utf8_lossy(&run.stderr).contains("Missing520"));
    assert_eq!(fs::read(db.join("1Cv8.1CD")).unwrap(), before);
}
#[test]
fn prepared_state_refuses_before_publication() {
    let dir = Scratch::new("prepared");
    let db = dir.db("prepared");
    let out = dir.0.join("output");
    let before = fs::read(db.join("1Cv8.1CD")).unwrap();
    let run = export(&db, "ТестРасширение", &out);
    assert_eq!(run.status.code(), Some(1));
    assert!(!out.exists());
    assert!(String::from_utf8_lossy(&run.stderr).contains("Подготовленное"));
    assert_eq!(fs::read(db.join("1Cv8.1CD")).unwrap(), before);
}
#[test]
fn corrupt_file_profile_refuses_without_output() {
    let dir = Scratch::new("corrupt");
    let db = dir.db("two-extensions");
    let out = dir.0.join("output");
    let path = db.join("1Cv8.1CD");
    let mut bytes = fs::read(&path).unwrap();
    bytes[8] = 9;
    fs::write(&path, &bytes).unwrap();
    let run = export(&db, "Second520", &out);
    assert_eq!(run.status.code(), Some(1));
    assert!(!out.exists());
    assert_eq!(fs::read(path).unwrap(), bytes);
}
#[test]
fn corrupt_registry_envelope_refuses_without_output() {
    let dir = Scratch::new("envelope");
    let db = dir.db("two-extensions");
    let out = dir.0.join("output");
    let path = db.join("1Cv8.1CD");
    let mut bytes = fs::read(&path).unwrap();
    let marker = [0x43, 0xc2, 0x9a, 0x14, 0x3d, 0x99, 0xfb, 0x85];
    let positions = bytes
        .windows(marker.len())
        .enumerate()
        .filter_map(|(i, w)| (w == marker).then_some(i))
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), 1);
    bytes[positions[0]] = 0xff;
    fs::write(&path, &bytes).unwrap();
    let run = export(&db, "Second520", &out);
    assert_eq!(run.status.code(), Some(1));
    assert!(!out.exists());
    assert_eq!(fs::read(path).unwrap(), bytes);
}
#[test]
fn existing_nonempty_output_is_preserved() {
    let dir = Scratch::new("output");
    let db = dir.db("two-extensions");
    let out = dir.0.join("output");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("sentinel"), b"keep").unwrap();
    let run = export(&db, "Second520", &out);
    assert!(!run.status.success());
    assert_eq!(fs::read(out.join("sentinel")).unwrap(), b"keep");
    assert_eq!(fs::read_dir(out).unwrap().count(), 1);
}

#[test]
fn corrupt_selected_cas_payload_refuses_without_output() {
    let dir = Scratch::new("cas");
    let db = dir.db("two-extensions");
    let out = dir.0.join("output");
    let path = db.join("1Cv8.1CD");
    let row = {
        let mut file = ibcmd_filedb::FileDb::open(&db).unwrap();
        file.config_rows_named(
            "CONFIGCAS",
            &std::collections::BTreeSet::from(["3d99fb8542039bb258a28a4b0b30bb6b8693e304".into()]),
        )
        .unwrap()
        .remove(0)
    };
    let mut bytes = fs::read(&path).unwrap();
    let positions = bytes
        .windows(16)
        .enumerate()
        .filter_map(|(i, w)| (w == &row.binary[..16]).then_some(i))
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), 1);
    bytes[positions[0] + 3] ^= 0x40;
    fs::write(&path, &bytes).unwrap();
    let run = export(&db, "Second520", &out);
    assert_eq!(run.status.code(), Some(1));
    assert!(!out.exists());
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn out_of_range_physical_part_number_is_refused() {
    let dir = Scratch::new("part-overflow");
    let db = dir.db("two-extensions");
    let out = dir.0.join("output");
    let part_offset = {
        let file = ibcmd_filedb::FileDb::open(&db).unwrap();
        let table = file.table("CONFIGCAS").unwrap();
        1 + table
            .fields
            .iter()
            .take_while(|f| f.name != "PARTNO")
            .map(|f| f.size())
            .sum::<usize>()
    };
    let path = db.join("1Cv8.1CD");
    let mut bytes = fs::read(&path).unwrap();
    let name = "3d99fb8542039bb258a28a4b0b30bb6b8693e304";
    let marker = (name.len() as u16)
        .to_le_bytes()
        .into_iter()
        .chain(name.encode_utf16().flat_map(u16::to_le_bytes))
        .collect::<Vec<_>>();
    let positions = bytes
        .windows(marker.len())
        .enumerate()
        .filter_map(|(i, w)| (w == marker).then_some(i))
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), 1);
    // The measured CONFIG layout starts FILENAME after its one-byte live flag.
    // N(10) can encode 2^32, which must not wrap into the supported part zero.
    let at = positions[0] - 1 + part_offset;
    bytes[at..at + 6].copy_from_slice(&[0x14, 0x29, 0x49, 0x67, 0x29, 0x60]);
    fs::write(&path, &bytes).unwrap();
    let run = export(&db, "Second520", &out);
    assert_eq!(run.status.code(), Some(1));
    assert!(!out.exists());
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn unsupported_80501_manifest_refuses_before_publication() {
    use std::io::Write;
    let dir = Scratch::new("unsupported-manifest");
    let db = dir.db("two-extensions");
    let out = dir.0.join("output");
    let old_root = "3d99fb8542039bb258a28a4b0b30bb6b8693e304";
    let row = ibcmd_filedb::FileDb::open(&db)
        .unwrap()
        .config_rows("CONFIGCAS")
        .unwrap()
        .into_iter()
        .find(|r| r.file_name == old_root)
        .unwrap();
    let mut plain = String::new();
    flate2::read::DeflateDecoder::new(row.binary.as_slice())
        .read_to_string(&mut plain)
        .unwrap();
    assert_eq!(plain.matches("80327").count(), 1);
    let changed = plain.replace("80327", "80501");
    let mut packed = (1..=9)
        .map(|level| {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(level));
            encoder.write_all(changed.as_bytes()).unwrap();
            encoder.finish().unwrap()
        })
        .min_by_key(Vec::len)
        .unwrap();
    assert!(packed.len() <= row.binary.len());
    // Keep the measured blob size/pointer intact; raw DEFLATE permits trailing
    // bytes, and the CAS digest covers those exact packed bytes as stored.
    packed.resize(row.binary.len(), 0);
    let mut decoded = String::new();
    flate2::read::DeflateDecoder::new(packed.as_slice())
        .read_to_string(&mut decoded)
        .unwrap();
    assert_eq!(decoded, changed);
    let new_root = format!("{:x}", sha1::Sha1::digest(&packed));
    let path = db.join("1Cv8.1CD");
    let mut bytes = fs::read(&path).unwrap();
    // Documented blob blocks carry at most 250 payload bytes between their
    // six-byte headers; find each original chunk before changing any bytes.
    let payload_positions = row
        .binary
        .chunks(250)
        .map(|chunk| {
            let positions = bytes
                .windows(chunk.len())
                .enumerate()
                .filter_map(|(i, w)| (w == chunk).then_some(i))
                .collect::<Vec<_>>();
            assert_eq!(positions.len(), 1);
            positions[0]
        })
        .collect::<Vec<_>>();
    for (at, chunk) in payload_positions.into_iter().zip(packed.chunks(250)) {
        bytes[at..at + chunk.len()].copy_from_slice(chunk);
    }
    let old_name = old_root
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let new_name = new_root
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let name_positions = bytes
        .windows(old_name.len())
        .enumerate()
        .filter_map(|(i, w)| (w == old_name).then_some(i))
        .collect::<Vec<_>>();
    assert_eq!(name_positions.len(), 1);
    let at = name_positions[0];
    bytes[at..at + new_name.len()].copy_from_slice(&new_name);
    let decode_hash = |hex: &str| {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>()
    };
    let marker = [0x43, 0xc2, 0x9a, 0x14]
        .into_iter()
        .chain(decode_hash(old_root))
        .collect::<Vec<_>>();
    let positions = bytes
        .windows(marker.len())
        .enumerate()
        .filter_map(|(i, w)| (w == marker).then_some(i))
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), 1);
    // Measured envelope: four-byte magic, then exactly the twenty-byte root.
    let at = positions[0] + 4;
    bytes[at..at + 20].copy_from_slice(&decode_hash(&new_root));
    fs::write(&path, &bytes).unwrap();
    let run = export(&db, "Second520", &out);
    assert_eq!(run.status.code(), Some(1));
    assert!(!out.exists());
    assert!(String::from_utf8_lossy(&run.stderr).contains("80501"));
    assert_eq!(fs::read(path).unwrap(), bytes);
}
