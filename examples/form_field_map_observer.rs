//! Local read-only observation of one raw-deflated Form body; no export or SQL.
use ibcmd_rs::mssql_dump::{MAX_FORM_OBSERVER_COMPRESSED_BYTES, observe_form_field_map_blob};
use std::io::{Read, Write};
fn run() -> Result<Vec<u8>, &'static str> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or("FORM_OBSERVER_ARGUMENTS")?;
    if args.next().is_some() {
        return Err("FORM_OBSERVER_ARGUMENTS");
    }
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| "FORM_OBSERVER_INPUT")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("FORM_OBSERVER_INPUT");
    }
    let file = std::fs::File::open(path).map_err(|_| "FORM_OBSERVER_INPUT")?;
    if !file
        .metadata()
        .map_err(|_| "FORM_OBSERVER_INPUT")?
        .is_file()
    {
        return Err("FORM_OBSERVER_INPUT");
    }
    let mut bytes = Vec::new();
    file.take((MAX_FORM_OBSERVER_COMPRESSED_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "FORM_OBSERVER_INPUT")?;
    let report = observe_form_field_map_blob(&bytes).map_err(|e| e.code())?;
    serde_json::to_vec(&report).map_err(|_| "FORM_OBSERVER_SERIALIZE")
}
fn main() {
    // Panic payloads from parsers must never expose a source value or path.
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(run).unwrap_or(Err("FORM_OBSERVER_PANIC"));
    match result {
        Ok(mut bytes) => {
            bytes.push(b'\n');
            if std::io::stdout().lock().write_all(&bytes).is_err() {
                eprintln!("FORM_OBSERVER_OUTPUT");
                std::process::exit(1);
            }
        }
        Err(code) => {
            eprintln!("{code}");
            std::process::exit(1);
        }
    }
}
