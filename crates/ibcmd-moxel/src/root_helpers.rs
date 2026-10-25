//! What the decoder took along from the root crate's `mssql_dump`.
//!
//! The two standard-picture identifiers moved here with the decoder, their
//! only reader. The functions are private copies of the root's helpers of
//! the same names, with the same bodies (`extract_base64_payload` is
//! `mssql_dump::source_assets`'s, the others are `mssql_dump`'s); the root
//! keeps its own for the rest of `mssql_dump`.
//!
//! The code came from the physical-adapter guard's slice, so the guard scans
//! this file too (`tools/validate-physical-adapter-policy.ps1`) and its
//! literals stay in the guard's baseline. The rest of the crate was outside
//! that slice before the move and stays outside it.

// Platform-level 1C standard pictures, not metadata UUIDs from one database.
pub(crate) const STD_PICTURE_INFORMATION_UUID: &str = "4b54770b-d069-4c0e-9b17-5cc2a01134d9";
pub(crate) const STD_PICTURE_SAVE_FILE_UUID: &str = "818ab7d0-4654-4542-bd5e-fd9d1352b5a1";

pub(crate) fn parse_uuid_field(value: &str) -> Option<String> {
    let value = value.trim();
    if is_uuid_text(value) {
        Some(value.to_string())
    } else {
        None
    }
}

pub(crate) fn parse_non_zero_uuid(value: &str) -> Option<String> {
    let uuid = parse_uuid_field(value)?;
    if uuid == "00000000-0000-0000-0000-000000000000" {
        None
    } else {
        Some(uuid)
    }
}

fn is_uuid_text(value: &str) -> bool {
    value.len() == 36 && value.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-')
}

pub(crate) fn xml_bool(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub(crate) fn extract_base64_payload(text: &str) -> Option<&str> {
    let prefix = "{#base64:";
    let start = text.find(prefix)? + prefix.len();
    let end = text[start..].find('}')? + start;
    Some(&text[start..end])
}
